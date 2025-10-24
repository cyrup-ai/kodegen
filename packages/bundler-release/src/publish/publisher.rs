//! Publishing orchestrator for workspace package publishing.
//!
//! This module coordinates the publishing of multiple packages in dependency order
//! with proper timing, error handling, and rollback capabilities.

use crate::error::{PublishError, Result};
use crate::publish::{CargoPublisher, PublishConfig, PublishResult, YankResult};
use crate::workspace::{DependencyGraph, PublishTier, SharedWorkspaceInfo};
use semver::Version;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Publishing orchestrator for workspace packages
#[derive(Debug)]
pub struct Publisher {
    /// Workspace information
    workspace: SharedWorkspaceInfo,
    /// Dependency graph for publish ordering
    dependency_graph: DependencyGraph,
    /// Cargo publisher for individual packages
    cargo_publisher: CargoPublisher,
    /// Publishing configuration
    config: PublisherConfig,
    /// State of current publishing operation
    publish_state: PublishState,
}

/// Configuration for the publishing orchestrator
#[derive(Debug, Clone)]
pub struct PublisherConfig {
    /// Delay between publishing packages (user requested 15 seconds)
    pub inter_package_delay: Duration,
    /// Whether to perform dry run validation first
    pub dry_run_first: bool,
    /// Whether to continue on non-critical failures
    pub continue_on_failure: bool,
    /// Maximum concurrent publishes within a tier
    pub max_concurrent_per_tier: usize,
    /// Registry to publish to
    pub registry: Option<String>,
    /// Whether to allow dirty working directory
    pub allow_dirty: bool,
    /// Additional cargo arguments
    pub additional_cargo_args: Vec<String>,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            inter_package_delay: Duration::from_secs(15), // User requested 15-second delays
            dry_run_first: false,
            continue_on_failure: false,
            max_concurrent_per_tier: 3,
            registry: None,
            allow_dirty: false,
            additional_cargo_args: Vec::new(),
        }
    }
}

/// State tracking for publishing operations
#[derive(Debug, Clone, Default)]
struct PublishState {
    /// Results of completed publishes
    completed_publishes: HashMap<String, PublishResult>,
    /// Packages that failed to publish
    failed_packages: HashMap<String, String>,
    /// Current tier being published
    current_tier: usize,
    /// Total tiers to publish
    total_tiers: usize,
    /// Start time of publishing operation
    start_time: Option<std::time::Instant>,
}

/// Result of complete publishing operation
#[derive(Debug, Clone)]
pub struct PublishingResult {
    /// Packages that were successfully published
    pub successful_publishes: HashMap<String, PublishResult>,
    /// Packages that failed to publish
    pub failed_packages: HashMap<String, String>,
    /// Total duration of publishing operation
    pub total_duration: Duration,
    /// Number of tiers processed
    pub tiers_processed: usize,
    /// Whether all packages were published successfully
    pub all_successful: bool,
}

/// Result of rollback operation
#[derive(Debug, Clone)]
pub struct RollbackResult {
    /// Packages that were successfully yanked
    pub yanked_packages: HashMap<String, YankResult>,
    /// Packages that failed to yank
    pub yank_failures: HashMap<String, String>,
    /// Total duration of rollback operation
    pub duration: Duration,
    /// Whether rollback was completely successful
    pub fully_successful: bool,
}

impl Publisher {
    /// Create a new publisher for the workspace
    pub fn new(workspace: SharedWorkspaceInfo) -> Result<Self> {
        let dependency_graph = DependencyGraph::build(&workspace)?;
        let cargo_publisher = CargoPublisher::new();
        let config = PublisherConfig::default();
        let publish_state = PublishState::default();

        Ok(Self {
            workspace,
            dependency_graph,
            cargo_publisher,
            config,
            publish_state,
        })
    }

    /// Create a publisher with custom configuration
    pub fn with_config(workspace: SharedWorkspaceInfo, config: PublisherConfig) -> Result<Self> {
        let dependency_graph = DependencyGraph::build(&workspace)?;
        let cargo_publisher = CargoPublisher::new();
        let publish_state = PublishState::default();

        Ok(Self {
            workspace,
            dependency_graph,
            cargo_publisher,
            config,
            publish_state,
        })
    }

    /// Publish all packages in dependency order
    pub async fn publish_all_packages(
        &mut self,
        runtime_config: &crate::cli::RuntimeConfig,
    ) -> Result<PublishingResult> {
        self.publish_state.start_time = Some(std::time::Instant::now());

        // Get publishing order
        let publish_order = self.dependency_graph.publish_order()?;
        self.publish_state.total_tiers = publish_order.tier_count();

        // Note: No upfront validation - cargo publish validates each package atomically.
        // Upfront validation would fail for packages with unpublished workspace dependencies.

        // Publish packages tier by tier
        for (tier_index, tier) in publish_order.tiers.iter().enumerate() {
            self.publish_state.current_tier = tier_index;

            match self.publish_tier(tier, runtime_config).await {
                Ok(()) => {
                    // Add delay between tiers (except after the last tier)
                    if tier_index < publish_order.tiers.len() - 1 {
                        sleep(self.config.inter_package_delay).await;
                    }
                }
                Err(e) if self.config.continue_on_failure => {
                    // Log error but continue with next tier
                    runtime_config
                        .warn(&format!("Tier {} failed but continuing: {}", tier_index, e));
                }
                Err(e) => {
                    // Fail fast - stop publishing
                    return Err(e);
                }
            }
        }

        let total_duration = self
            .publish_state
            .start_time
            .map(|start| start.elapsed())
            .unwrap_or_default();

        let all_successful = self.publish_state.failed_packages.is_empty();

        Ok(PublishingResult {
            successful_publishes: self.publish_state.completed_publishes.clone(),
            failed_packages: self.publish_state.failed_packages.clone(),
            total_duration,
            tiers_processed: self.publish_state.current_tier + 1,
            all_successful,
        })
    }

    /// Publish a single tier of packages
    async fn publish_tier(
        &mut self,
        tier: &PublishTier,
        runtime_config: &crate::cli::RuntimeConfig,
    ) -> Result<()> {
        let publish_config = self.create_publish_config();

        // Handle single package or parallel publishing
        if tier.packages.len() == 1 {
            // Single package - publish directly
            let package_name = &tier.packages[0];
            self.publish_single_package(package_name, &publish_config, runtime_config)
                .await?;
        } else {
            // Multiple packages - publish with controlled concurrency
            self.publish_packages_concurrently(&tier.packages, &publish_config, runtime_config)
                .await?;

            // Add delay after concurrent publishes to avoid rate limiting the next tier
            sleep(self.config.inter_package_delay).await;
        }

        Ok(())
    }

    /// Publish a single package
    async fn publish_single_package(
        &mut self,
        package_name: &str,
        publish_config: &PublishConfig,
        runtime_config: &crate::cli::RuntimeConfig,
    ) -> Result<()> {
        let package_info = self.workspace.get_package(package_name)?;

        runtime_config.progress(&format!(
            "Publishing {} v{}...",
            package_name, package_info.version
        ));

        match self
            .cargo_publisher
            .publish_package(package_info, publish_config)
            .await
        {
            Ok(result) => {
                runtime_config.success(&result.summary());
                self.publish_state
                    .completed_publishes
                    .insert(package_name.to_string(), result);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to publish {}: {}", package_name, e);
                self.publish_state
                    .failed_packages
                    .insert(package_name.to_string(), error_msg.clone());
                Err(PublishError::PublishFailed {
                    package: package_name.to_string(),
                    reason: error_msg,
                }
                .into())
            }
        }
    }

    /// Publish multiple packages concurrently (within a tier)
    async fn publish_packages_concurrently(
        &mut self,
        package_names: &[String],
        publish_config: &PublishConfig,
        runtime_config: &crate::cli::RuntimeConfig,
    ) -> Result<()> {
        use std::sync::Arc;
        use tokio::sync::{Semaphore, mpsc};

        // Unbounded channel for results (bounded would risk deadlock with semaphore)
        let (tx, mut rx) = mpsc::unbounded_channel();
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_per_tier));

        // Spawn all tasks (they'll complete at different times)
        for package_name in package_names {
            let package_info = self.workspace.get_package(package_name)?.clone();
            let publisher = self.cargo_publisher.clone();
            let config = publish_config.clone();
            let semaphore = Arc::clone(&semaphore);
            let tx = tx.clone();
            let package_name = package_name.clone();

            tokio::spawn(async move {
                // Acquire semaphore permit with proper error handling
                let permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(e) => {
                        // Semaphore closed - send error through channel
                        let error = PublishError::PublishFailed {
                            package: package_name.clone(),
                            reason: format!("Semaphore acquire failed: {}", e),
                        };
                        let _ = tx.send((package_name, Err(error.into())));
                        return;
                    }
                };

                let result = publisher.publish_package(&package_info, &config).await;

                // Send result through channel (non-blocking)
                let _ = tx.send((package_name, result));

                // Explicitly drop permit to release semaphore slot
                drop(permit);
            });
        }

        // Drop original sender so rx.recv() knows when all tasks are done
        drop(tx);

        // Process results as they complete (incremental progress!)
        while let Some((package_name, result)) = rx.recv().await {
            match result {
                Ok(publish_result) => {
                    runtime_config.success(&publish_result.summary());
                    self.publish_state
                        .completed_publishes
                        .insert(package_name, publish_result);
                }
                Err(e) => {
                    let error_msg = format!("Failed to publish {}: {}", package_name, e);
                    self.publish_state
                        .failed_packages
                        .insert(package_name.clone(), error_msg.clone());

                    if !self.config.continue_on_failure {
                        return Err(PublishError::PublishFailed {
                            package: package_name,
                            reason: error_msg,
                        }
                        .into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Rollback published packages by yanking them
    pub async fn rollback_published_packages(
        &self,
        runtime_config: &crate::cli::RuntimeConfig,
    ) -> Result<RollbackResult> {
        let start_time = std::time::Instant::now();
        let mut yanked_packages = HashMap::new();
        let mut yank_failures = HashMap::new();
        let publish_config = self.create_publish_config();

        // Yank packages in reverse dependency order
        let publish_order = self.dependency_graph.publish_order()?;
        let mut packages_to_yank: Vec<_> = publish_order.ordered_packages().collect();
        packages_to_yank.reverse();

        for package_name in packages_to_yank {
            if let Some(publish_result) = self.publish_state.completed_publishes.get(package_name) {
                runtime_config.progress(&format!(
                    "Yanking {} v{}...",
                    package_name, publish_result.version
                ));

                match self
                    .cargo_publisher
                    .yank_package(package_name, &publish_result.version, &publish_config)
                    .await
                {
                    Ok(yank_result) => {
                        runtime_config.success(&yank_result.format_result());
                        yanked_packages.insert(package_name.to_string(), yank_result);
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to yank {}: {}", package_name, e);
                        runtime_config.error(&error_msg);
                        yank_failures.insert(package_name.to_string(), error_msg);
                    }
                }
            }
        }

        let duration = start_time.elapsed();
        let fully_successful = yank_failures.is_empty();

        Ok(RollbackResult {
            yanked_packages,
            yank_failures,
            duration,
            fully_successful,
        })
    }

    /// Create publish configuration from publisher config
    fn create_publish_config(&self) -> PublishConfig {
        PublishConfig {
            registry: self.config.registry.clone(),
            dry_run_first: false, // Already handled at orchestrator level
            allow_dirty: self.config.allow_dirty,
            additional_args: self.config.additional_cargo_args.clone(),
            token: None, // Use cargo login
        }
    }

    /// Check if any packages are already published
    pub async fn check_already_published(&self) -> Result<HashMap<String, bool>> {
        let mut results = HashMap::new();

        for (package_name, package_info) in &self.workspace.packages {
            let version =
                Version::parse(&package_info.version).map_err(|e| PublishError::PublishFailed {
                    package: package_name.clone(),
                    reason: format!("Invalid version: {}", e),
                })?;

            let is_published = self
                .cargo_publisher
                .is_package_published(package_name, &version)
                .await?;
            results.insert(package_name.clone(), is_published);
        }

        Ok(results)
    }

    /// Get current publishing progress
    pub fn get_progress(&self) -> PublishProgress {
        let total_packages = self.workspace.packages.len();
        let completed_packages = self.publish_state.completed_publishes.len();
        let failed_packages = self.publish_state.failed_packages.len();
        let remaining_packages = total_packages - completed_packages - failed_packages;

        PublishProgress {
            total_packages,
            completed_packages,
            failed_packages,
            remaining_packages,
            current_tier: self.publish_state.current_tier,
            total_tiers: self.publish_state.total_tiers,
            elapsed_time: self.publish_state.start_time.map(|start| start.elapsed()),
        }
    }

    /// Update publisher configuration
    pub fn set_config(&mut self, config: PublisherConfig) {
        self.config = config;
    }

    /// Get publisher configuration
    pub fn config(&self) -> &PublisherConfig {
        &self.config
    }

    /// Clear publishing state
    pub fn clear_state(&mut self) {
        self.publish_state = PublishState::default();
    }
}

/// Current progress of publishing operation
#[derive(Debug, Clone)]
pub struct PublishProgress {
    /// Total number of packages to publish
    pub total_packages: usize,
    /// Number of packages successfully published
    pub completed_packages: usize,
    /// Number of packages that failed to publish
    pub failed_packages: usize,
    /// Number of packages remaining to publish
    pub remaining_packages: usize,
    /// Current tier being processed
    pub current_tier: usize,
    /// Total number of tiers
    pub total_tiers: usize,
    /// Elapsed time since publishing started
    pub elapsed_time: Option<Duration>,
}

impl PublishingResult {
    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.successful_publishes.len() + self.failed_packages.len();
        if total == 0 {
            0.0
        } else {
            (self.successful_publishes.len() as f64 / total as f64) * 100.0
        }
    }

    /// Format result summary
    pub fn format_summary(&self) -> String {
        let status = if self.all_successful { "✅" } else { "⚠️" };
        format!(
            "{} Publishing completed: {}/{} packages successful ({:.1}%) in {:.2}s",
            status,
            self.successful_publishes.len(),
            self.successful_publishes.len() + self.failed_packages.len(),
            self.success_rate(),
            self.total_duration.as_secs_f64()
        )
    }

    /// Format detailed report
    pub fn format_report(&self) -> String {
        let mut report = self.format_summary();
        report.push('\n');

        if !self.successful_publishes.is_empty() {
            report.push_str("\n📦 Successfully Published:\n");
            for result in self.successful_publishes.values() {
                report.push_str(&format!("  ✅ {}\n", result.summary()));
            }
        }

        if !self.failed_packages.is_empty() {
            report.push_str("\n❌ Failed Packages:\n");
            for (package, error) in &self.failed_packages {
                report.push_str(&format!("  ❌ {}: {}\n", package, error));
            }
        }

        report
    }
}

impl RollbackResult {
    /// Format rollback summary
    pub fn format_summary(&self) -> String {
        let status = if self.fully_successful {
            "✅"
        } else {
            "⚠️"
        };
        format!(
            "{} Rollback completed: {}/{} packages yanked in {:.2}s",
            status,
            self.yanked_packages.len(),
            self.yanked_packages.len() + self.yank_failures.len(),
            self.duration.as_secs_f64()
        )
    }
}

impl PublishProgress {
    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_packages == 0 {
            100.0
        } else {
            ((self.completed_packages + self.failed_packages) as f64 / self.total_packages as f64)
                * 100.0
        }
    }

    /// Format progress for display
    pub fn format_progress(&self) -> String {
        let elapsed = self
            .elapsed_time
            .map(|d| format!(" ({}s)", d.as_secs()))
            .unwrap_or_default();

        format!(
            "📊 Progress: {:.1}% ({}/{} packages) - Tier {}/{}{} - {} remaining",
            self.completion_percentage(),
            self.completed_packages,
            self.total_packages,
            self.current_tier + 1,
            self.total_tiers,
            elapsed,
            self.remaining_packages
        )
    }
}
