//! Cargo publishing operations with retry logic and error handling.
//!
//! This module provides robust cargo publish operations with rate limiting,
//! retry logic, and comprehensive error handling for crates.io publishing.

use crate::error::{PublishError, Result};
use crate::workspace::PackageInfo;
use regex::Regex;
use semver::Version;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::{sleep, timeout};

/// Cargo publisher with retry logic and rate limiting
#[derive(Debug, Clone)]
pub struct CargoPublisher {
    /// Maximum number of retry attempts
    max_retries: usize,
    /// Base delay between retries (exponential backoff)
    base_retry_delay: Duration,
    /// Timeout for individual operations
    operation_timeout: Duration,
    /// Whether to use dry-run validation
    dry_run_validation: bool,
}

impl Default for CargoPublisher {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_retry_delay: Duration::from_secs(5),
            operation_timeout: Duration::from_secs(300), // 5 minutes
            dry_run_validation: true,
        }
    }
}

/// Result of a cargo publish operation
#[derive(Debug, Clone)]
pub struct PublishResult {
    /// Package name that was published
    pub package_name: String,
    /// Version that was published
    pub version: Version,
    /// Duration of the publish operation
    pub duration: Duration,
    /// Number of retry attempts made
    pub retry_attempts: usize,
    /// Any warnings from cargo publish
    pub warnings: Vec<String>,
    /// Whether this was a dry run
    pub dry_run: bool,
}

/// Result of a cargo yank operation
#[derive(Debug, Clone)]
pub struct YankResult {
    /// Package name that was yanked
    pub package_name: String,
    /// Version that was yanked
    pub version: Version,
    /// Duration of the yank operation
    pub duration: Duration,
    /// Whether the package was successfully yanked
    pub success: bool,
}

/// Configuration for publishing operations
#[derive(Debug, Clone, Default)]
pub struct PublishConfig {
    /// Registry to publish to (defaults to crates.io)
    pub registry: Option<String>,
    /// Whether to perform dry run validation first
    pub dry_run_first: bool,
    /// Whether to allow dirty working directory
    pub allow_dirty: bool,
    /// Additional cargo publish arguments
    pub additional_args: Vec<String>,
    /// Token for authentication (if not using cargo login)
    pub token: Option<String>,
}

impl CargoPublisher {
    /// Create a new cargo publisher with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cargo publisher with custom retry configuration
    pub fn with_retry_config(max_retries: usize, base_delay: Duration, timeout: Duration) -> Self {
        Self {
            max_retries,
            base_retry_delay: base_delay,
            operation_timeout: timeout,
            dry_run_validation: true,
        }
    }

    /// Publish a package to crates.io
    pub async fn publish_package(
        &self,
        package_info: &PackageInfo,
        config: &PublishConfig,
    ) -> Result<PublishResult> {
        let start_time = std::time::Instant::now();
        let package_name = &package_info.name;
        let version =
            Version::parse(&package_info.version).map_err(|e| PublishError::PublishFailed {
                package: package_name.clone(),
                reason: format!("Invalid version '{}': {}", package_info.version, e),
            })?;

        let mut warnings = Vec::new();
        let mut retry_attempts = 0;

        // Note: cargo publish performs validation automatically before publishing.
        // We don't validate separately because it would fail for packages with
        // unpublished workspace dependencies (checks crates.io, not local workspace).
        // The publish operation is atomic - if validation fails, nothing gets published.

        // Attempt publish with retry logic
        self.retry_with_backoff(
            || self.attempt_publish(package_info, config),
            &mut retry_attempts,
            &mut warnings,
        )
        .await?;

        let duration = start_time.elapsed();

        Ok(PublishResult {
            package_name: package_name.clone(),
            version,
            duration,
            retry_attempts,
            warnings,
            dry_run: false,
        })
    }

    /// Perform dry run validation of a package
    pub async fn validate_package_dry_run(
        &self,
        package_info: &PackageInfo,
        config: &PublishConfig,
    ) -> Result<PublishResult> {
        let start_time = std::time::Instant::now();
        let package_name = &package_info.name;
        let version =
            Version::parse(&package_info.version).map_err(|e| PublishError::DryRunFailed {
                package: package_name.clone(),
                reason: format!("Invalid version '{}': {}", package_info.version, e),
            })?;

        let mut cmd = self.build_publish_command(package_info, config);
        cmd.arg("--dry-run");

        let output = timeout(self.operation_timeout, cmd.output())
            .await
            .map_err(|_| PublishError::DryRunFailed {
                package: package_name.clone(),
                reason: "Dry run timed out".to_string(),
            })?
            .map_err(|e| PublishError::DryRunFailed {
                package: package_name.clone(),
                reason: format!("Failed to execute cargo publish: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PublishError::DryRunFailed {
                package: package_name.clone(),
                reason: stderr.to_string(),
            }
            .into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let warnings = self.extract_warnings(&stdout);

        let duration = start_time.elapsed();

        Ok(PublishResult {
            package_name: package_name.clone(),
            version,
            duration,
            retry_attempts: 0,
            warnings,
            dry_run: true,
        })
    }

    /// Yank a published package version
    pub async fn yank_package(
        &self,
        package_name: &str,
        version: &Version,
        config: &PublishConfig,
    ) -> Result<YankResult> {
        let start_time = std::time::Instant::now();

        let mut cmd = Command::new("cargo");
        cmd.arg("yank")
            .arg("--vers")
            .arg(version.to_string())
            .arg(package_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add registry if specified
        if let Some(ref registry) = config.registry {
            cmd.arg("--registry").arg(registry);
        }

        // Add token if specified
        if let Some(ref token) = config.token {
            cmd.arg("--token").arg(token);
        }

        let output = timeout(self.operation_timeout, cmd.output())
            .await
            .map_err(|_| PublishError::YankFailed {
                package: package_name.to_string(),
                version: version.to_string(),
                reason: "Yank operation timed out".to_string(),
            })?
            .map_err(|e| PublishError::YankFailed {
                package: package_name.to_string(),
                version: version.to_string(),
                reason: format!("Failed to execute cargo yank: {}", e),
            })?;

        let success = output.status.success();
        if !success {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PublishError::YankFailed {
                package: package_name.to_string(),
                version: version.to_string(),
                reason: stderr.to_string(),
            }
            .into());
        }

        let duration = start_time.elapsed();

        Ok(YankResult {
            package_name: package_name.to_string(),
            version: version.clone(),
            duration,
            success,
        })
    }

    /// Attempt a single publish operation
    async fn attempt_publish(
        &self,
        package_info: &PackageInfo,
        config: &PublishConfig,
    ) -> Result<()> {
        let mut cmd = self.build_publish_command(package_info, config);

        let output = timeout(self.operation_timeout, cmd.output())
            .await
            .map_err(|_| PublishError::PublishFailed {
                package: package_info.name.clone(),
                reason: "Publish operation timed out".to_string(),
            })?
            .map_err(|e| PublishError::PublishFailed {
                package: package_info.name.clone(),
                reason: format!("Failed to execute cargo publish: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check for specific error types
            if stderr.contains("rate limit") || stderr.contains("too many requests") {
                let retry_after = self.parse_retry_after(&stderr).unwrap_or(60);
                return Err(PublishError::RateLimitExceeded {
                    retry_after_seconds: retry_after,
                }
                .into());
            }

            if stderr.contains("already published") {
                return Err(PublishError::AlreadyPublished {
                    package: package_info.name.clone(),
                    version: package_info.version.clone(),
                }
                .into());
            }

            if stderr.contains("authentication") || stderr.contains("unauthorized") {
                return Err(PublishError::AuthenticationError.into());
            }

            return Err(PublishError::PublishFailed {
                package: package_info.name.clone(),
                reason: stderr.to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Build cargo publish command
    fn build_publish_command(&self, package_info: &PackageInfo, config: &PublishConfig) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.arg("publish")
            .arg("--manifest-path")
            .arg(&package_info.cargo_toml_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add registry if specified
        if let Some(ref registry) = config.registry {
            cmd.arg("--registry").arg(registry);
        }

        // Add token if specified
        if let Some(ref token) = config.token {
            cmd.arg("--token").arg(token);
        }

        // Add allow dirty if specified
        if config.allow_dirty {
            cmd.arg("--allow-dirty");
        }

        // Add additional arguments
        for arg in &config.additional_args {
            cmd.arg(arg);
        }

        cmd
    }

    /// Retry operation with exponential backoff
    async fn retry_with_backoff<F, Fut>(
        &self,
        mut operation: F,
        retry_attempts: &mut usize,
        warnings: &mut Vec<String>,
    ) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut delay = self.base_retry_delay;

        for attempt in 0..=self.max_retries {
            *retry_attempts = attempt;

            match operation().await {
                Ok(()) => return Ok(()),
                Err(e) if attempt == self.max_retries => return Err(e),
                Err(e) => {
                    // Check if this is a retryable error
                    if !self.is_retryable_error(&e) {
                        return Err(e);
                    }

                    // Check if this is a rate limit error with specific retry-after
                    let wait_time = if let crate::error::ReleaseError::Publish(
                        PublishError::RateLimitExceeded {
                            retry_after_seconds,
                        },
                    ) = &e
                    {
                        Duration::from_secs(*retry_after_seconds)
                    } else {
                        // Use exponential backoff for other errors
                        let current_delay = delay;
                        delay = std::cmp::min(delay * 2, Duration::from_secs(300)); // Cap at 5 minutes
                        current_delay
                    };

                    warnings.push(format!(
                        "Attempt {} failed: {}. Retrying in {:.1}s...",
                        attempt + 1,
                        e,
                        wait_time.as_secs_f64()
                    ));

                    sleep(wait_time).await;
                }
            }
        }

        unreachable!("Loop should have returned or errored")
    }

    /// Parse retry-after seconds from cargo stderr output
    fn parse_retry_after(&self, stderr: &str) -> Option<u64> {
        let re = Regex::new(r"try again in (\d+) seconds").ok()?;
        let captures = re.captures(stderr)?;
        captures.get(1)?.as_str().parse().ok()
    }

    /// Check if an error is retryable
    fn is_retryable_error(&self, error: &crate::error::ReleaseError) -> bool {
        match error {
            crate::error::ReleaseError::Publish(publish_error) => {
                matches!(
                    publish_error,
                    PublishError::NetworkError { .. } | PublishError::RateLimitExceeded { .. }
                )
            }
            _ => false,
        }
    }

    /// Extract warnings from cargo output
    fn extract_warnings(&self, output: &str) -> Vec<String> {
        output
            .lines()
            .filter(|line| line.contains("warning:"))
            .map(|line| line.trim().to_string())
            .collect()
    }

    /// Check if a package is already published
    pub async fn is_package_published(
        &self,
        package_name: &str,
        version: &Version,
    ) -> Result<bool> {
        let mut cmd = Command::new("cargo");
        cmd.arg("search")
            .arg(package_name)
            .arg("--limit")
            .arg("1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = timeout(Duration::from_secs(30), cmd.output())
            .await
            .map_err(|_| PublishError::NetworkError {
                reason: "Search operation timed out".to_string(),
            })?
            .map_err(|e| PublishError::NetworkError {
                reason: format!("Failed to execute cargo search: {}", e),
            })?;

        if !output.status.success() {
            return Ok(false); // If search fails, assume not published
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse search output to check if exact version exists
        for line in stdout.lines() {
            if line.starts_with(package_name) && line.contains(&format!("= \"{}\"", version)) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get published versions of a package
    pub async fn get_published_versions(&self, package_name: &str) -> Result<Vec<Version>> {
        let mut cmd = Command::new("cargo");
        cmd.arg("search")
            .arg(package_name)
            .arg("--limit")
            .arg("1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = timeout(Duration::from_secs(30), cmd.output())
            .await
            .map_err(|_| PublishError::NetworkError {
                reason: "Search operation timed out".to_string(),
            })?
            .map_err(|e| PublishError::NetworkError {
                reason: format!("Failed to execute cargo search: {}", e),
            })?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut versions = Vec::new();

        // Parse versions from search output
        for line in stdout.lines() {
            if line.starts_with(package_name)
                && let Some(version_part) = line.split("= \"").nth(1)
                && let Some(version_str) = version_part.split('"').next()
                && let Ok(version) = Version::parse(version_str)
            {
                versions.push(version);
            }
        }

        Ok(versions)
    }

    /// Update publisher configuration
    pub fn set_max_retries(&mut self, max_retries: usize) {
        self.max_retries = max_retries;
    }

    /// Update retry delay
    pub fn set_retry_delay(&mut self, delay: Duration) {
        self.base_retry_delay = delay;
    }

    /// Update operation timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.operation_timeout = timeout;
    }

    /// Enable or disable dry run validation
    pub fn set_dry_run_validation(&mut self, enabled: bool) {
        self.dry_run_validation = enabled;
    }
}

impl PublishResult {
    /// Check if the publish was successful
    pub fn is_successful(&self) -> bool {
        !self.dry_run // Real publishes are successful if they complete
    }

    /// Get a summary of the publish operation
    pub fn summary(&self) -> String {
        let status = if self.dry_run {
            "validated"
        } else {
            "published"
        };
        let retry_info = if self.retry_attempts > 0 {
            format!(" (after {} retries)", self.retry_attempts)
        } else {
            String::new()
        };

        format!(
            "Successfully {} {}@{} in {:.2}s{}",
            status,
            self.package_name,
            self.version,
            self.duration.as_secs_f64(),
            retry_info
        )
    }

    /// Format detailed report
    pub fn format_report(&self) -> String {
        let mut report = self.summary();

        if !self.warnings.is_empty() {
            report.push_str("\nWarnings:\n");
            for warning in &self.warnings {
                report.push_str(&format!("  - {}\n", warning));
            }
        }

        report
    }
}

impl YankResult {
    /// Format yank result for display
    pub fn format_result(&self) -> String {
        if self.success {
            format!(
                "✅ Successfully yanked {}@{} in {:.2}s",
                self.package_name,
                self.version,
                self.duration.as_secs_f64()
            )
        } else {
            format!("❌ Failed to yank {}@{}", self.package_name, self.version)
        }
    }
}
