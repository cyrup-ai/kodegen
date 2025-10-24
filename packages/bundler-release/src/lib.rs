//! # Cyrup Release
//!
//! Production-quality release management for Rust workspaces.
//!
//! This crate provides atomic release operations with proper error handling,
//! automatic internal dependency version synchronization, and rollback capabilities
//! including crate yanking for published packages.
//!
//! ## Features
//!
//! - **Atomic Operations**: All release steps succeed or all rollback
//! - **Version Synchronization**: Automatic internal dependency version management  
//! - **Git Integration**: Pure Rust git operations using gix (no CLI dependencies)
//! - **Resume Capability**: Continue interrupted releases from checkpoints
//! - **Rollback Support**: Undo git operations and yank published crates
//! - **Dependency Ordering**: Publish packages in correct dependency order
//!
//! ## Usage
//!
//! ```bash
//! cyrup_release patch          # Bump patch version and publish
//! cyrup_release minor --dry    # Dry run minor version bump
//! cyrup_release rollback       # Rollback failed release
//! cyrup_release resume         # Resume interrupted release
//! ```

// SECURITY: Use deny instead of forbid to allow cli/docker.rs to use unsafe code
// for libc::getuid() and libc::getgid() calls (required for Docker container security)
#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// Core modules
pub mod bundler;
pub mod cli;
pub mod error;
pub mod git;
pub mod github;
pub mod publish;
pub mod state;
pub mod version;
pub mod workspace;

// Re-export main types for public API
pub use bundler::{BundleSettings, BundledArtifact, Bundler, PackageType};
pub use cli::{Args, Command};
pub use error::{CliError, ReleaseError, Result};
pub use git::{GitManager, GitOperations};
pub use publish::Publisher;
pub use state::{ReleaseState, StateManager};
pub use version::{VersionBump, VersionManager};
pub use workspace::{DependencyGraph, PublishOrder, WorkspaceInfo};

use std::path::PathBuf;

/// Configuration for release operations
#[derive(Debug, Clone)]
pub struct ReleaseConfig {
    /// Skip workspace validation
    pub skip_validation: bool,
    /// Allow dirty git working directory
    pub allow_dirty: bool,
    /// Skip pushing to remote
    pub no_push: bool,
    /// Registry to publish to (default: crates.io)
    pub registry: Option<String>,
    /// Delay between package publishes (seconds)
    pub package_delay: u64,
    /// Maximum retry attempts for publish
    pub max_retries: usize,
    /// Operation timeout (seconds)
    pub timeout: u64,
    /// Create GitHub release
    pub github_release: bool,
    /// GitHub repository (owner/repo)
    pub github_repo: Option<String>,
    /// Create draft GitHub release
    pub github_draft: bool,
    /// Path to release notes file
    pub release_notes: Option<PathBuf>,
    /// Create platform bundles
    pub with_bundles: bool,
    /// Upload bundles to GitHub release
    pub upload_bundles: bool,
    /// Keep temporary clone for debugging (don't auto-cleanup)
    pub keep_temp: bool,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            skip_validation: false,
            allow_dirty: false,
            no_push: false,
            registry: None,
            package_delay: 5,
            max_retries: 3,
            timeout: 300,
            github_release: true, // Always create GitHub releases unless --no-github-release
            github_repo: None,
            github_draft: false,
            release_notes: None,
            with_bundles: true, // Always create platform bundles unless --no-bundles
            upload_bundles: true, // Always upload bundles unless --no-upload-bundles
            keep_temp: false,
        }
    }
}

/// Result of a release operation
#[derive(Debug, Clone)]
pub struct ReleaseResult {
    /// New version number
    pub version: semver::Version,
    /// List of successfully published packages
    pub published_packages: Vec<String>,
    /// Git commit SHA (if committed)
    pub git_commit: Option<String>,
    /// Git tag name (if tagged)
    pub git_tag: Option<String>,
    /// GitHub release URL (if created)
    pub github_release: Option<String>,
    /// Number of artifacts signed
    pub artifacts_signed: usize,
    /// Number of bundles created
    pub bundles_created: usize,
}

/// Result of rollback operation
#[derive(Debug, Clone)]
pub struct RollbackResult {
    /// List of packages that were yanked
    pub packages_yanked: Vec<String>,
    /// Whether git operations were reverted
    pub git_reverted: bool,
    /// Whether GitHub release was deleted
    pub github_deleted: bool,
}

/* LEGACY HELPER FUNCTIONS - NOT USED IN ACTIVE CODE PATH
The active implementations are in cli/commands/temp_clone.rs */

/// Get origin remote URL from repository
#[allow(dead_code)]
async fn get_origin_url(repo_path: &std::path::Path) -> Result<String> {
    let repo = kodegen_tools_git::open_repo(repo_path)
        .await
        .map_err(|_| ReleaseError::Git(error::GitError::NotRepository))?
        .map_err(|_| ReleaseError::Git(error::GitError::NotRepository))?;

    let remotes = kodegen_tools_git::list_remotes(&repo).await.map_err(|e| {
        ReleaseError::Git(error::GitError::RemoteOperationFailed {
            operation: "list_remotes".to_string(),
            reason: e.to_string(),
        })
    })?;
    let origin = remotes.iter().find(|r| r.name == "origin").ok_or_else(|| {
        ReleaseError::Git(error::GitError::RemoteOperationFailed {
            operation: "find_origin".to_string(),
            reason: "No origin remote found. Please configure an 'origin' remote.".to_string(),
        })
    })?;

    Ok(origin.fetch_url.clone())
}

/// Clone main branch to temporary directory for isolated release
#[allow(dead_code)]
async fn clone_main_to_temp() -> Result<PathBuf> {
    use std::path::Path;

    // Get origin URL from current repository
    let remote_url = get_origin_url(Path::new(".")).await?;

    // Create unique temp directory with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| {
            ReleaseError::Cli(error::CliError::ExecutionFailed {
                command: "get_timestamp".to_string(),
                reason: e.to_string(),
            })
        })?
        .as_secs();

    let temp_dir = std::env::temp_dir().join(format!("kodegen-release-{}", timestamp));

    // Clone main branch to temp
    println!("🔄 Cloning main branch to isolated environment...");
    println!("   Temp location: {}", temp_dir.display());

    let clone_opts = kodegen_tools_git::CloneOpts::new(remote_url, temp_dir.clone()).branch("main");

    kodegen_tools_git::clone_repo(clone_opts)
        .await
        .map_err(|e| {
            ReleaseError::Git(error::GitError::RemoteOperationFailed {
                operation: "clone_repo".to_string(),
                reason: format!("Failed to clone repository: {}", e),
            })
        })?
        .map_err(|e| {
            ReleaseError::Git(error::GitError::RemoteOperationFailed {
                operation: "clone_repo".to_string(),
                reason: e.to_string(),
            })
        })?;

    println!("✅ Clone complete");

    Ok(temp_dir)
}

/* LEGACY CODE - NOT USED IN ACTIVE CODE PATH
   This ReleaseManager implementation is preserved for reference but is not used.
   The active implementation is in cli/commands/release/impl.rs
   Commented out to avoid compilation errors from outdated API usage.

/// Main release orchestrator that coordinates all operations
pub struct ReleaseManager {
    workspace: workspace::SharedWorkspaceInfo,
    git: GitManager,
    publisher: Publisher,
    state: StateManager,
}

impl ReleaseManager {
    /// Create a new release manager for the current workspace
    pub async fn new() -> Result<Self> {
        use std::sync::Arc;
        let workspace = Arc::new(WorkspaceInfo::analyze(".")?);
        let git = GitManager::new(".").await?;
        let publisher = Publisher::new(workspace.clone())?;
        let state = StateManager::new(".cyrup_release_state.json")?;

        Ok(Self {
            workspace,
            git,
            publisher,
            state,
        })
    }

    /// Execute a release with the specified version bump
    ///
    /// This performs the release in an isolated temporary clone of the repository,
    /// ensuring the user's working directory is never modified.
    pub async fn release(&mut self, bump: VersionBump, config: ReleaseConfig) -> Result<ReleaseResult> {
        // Clone main branch to isolated temp directory
        let temp_dir = clone_main_to_temp().await?;

        println!("🚀 Starting release in isolated environment");
        println!("   Working directory untouched - all operations in temp clone");

        // Perform release in temp clone
        let result = self.perform_release_in_temp(&temp_dir, bump, config.clone()).await;

        // Always cleanup temp directory (unless debugging)
        if !config.keep_temp {
            if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
                eprintln!("⚠️  Warning: Failed to cleanup temp directory: {}", e);
                eprintln!("   You may need to manually remove: {}", temp_dir.display());
            } else {
                println!("✅ Temp clone cleaned up");
            }
        } else {
            println!("🔍 Temp clone kept for debugging at: {}", temp_dir.display());
        }

        result
    }

    /// Perform release operations in a temporary clone directory
    async fn perform_release_in_temp(
        &mut self,
        temp_dir: &std::path::Path,
        bump: VersionBump,
        config: ReleaseConfig,
    ) -> Result<ReleaseResult> {
        use crate::error::CliError;
        use crate::git::GitConfig;
        use crate::publish::PublisherConfig;
        use crate::state::{ReleasePhase, create_state_manager_at, has_active_release_at};
        use crate::workspace::WorkspaceValidator;

        // Phase 0: Pre-validation
        let state_path = temp_dir.join(".cyrup_release_state.json");
        if has_active_release_at(&state_path) {
            return Err(ReleaseError::State(crate::error::StateError::SaveFailed {
                reason: "Another release is in progress. Use 'resume' or 'cleanup' first".to_string(),
            }));
        }

        // Re-analyze workspace from temp clone
        let workspace = std::sync::Arc::new(WorkspaceInfo::analyze(temp_dir)?);

        // Validate workspace if not skipped
        if !config.skip_validation {
            let validator = WorkspaceValidator::new(workspace.clone())?;
            let validation = validator.validate().await?;

            if !validation.success {
                return Err(ReleaseError::Workspace(crate::error::WorkspaceError::InvalidStructure {
                    reason: format!("Validation failed: {}", validation.critical_errors.join(", ")),
                }));
            }
        }

        // Initialize version manager with temp workspace
        let mut version_manager = VersionManager::new(workspace.clone());

        // Configure git manager for temp clone
        let git_config = GitConfig {
            default_remote: "origin".to_string(),
            annotated_tags: true,
            auto_push_tags: !config.no_push,
            ..Default::default()
        };
        let mut git = GitManager::with_config(temp_dir, git_config).await?;

        // Configure publisher for temp workspace
        let publisher_config = PublisherConfig {
            inter_package_delay: Duration::from_secs(config.package_delay),
            registry: config.registry.clone(),
            max_concurrent_per_tier: 1,
            ..Default::default()
        };
        let mut publisher = Publisher::with_config(workspace.clone(), publisher_config)?;

        // Create release state
        let current_version = version_manager.current_version()?;
        let bumper = crate::version::VersionBumper::from_version(current_version.clone());
        let new_version = bumper.bump(bump.clone())?;

        let release_config = state::ReleaseConfig {
            dry_run_first: false,
            push_to_remote: !config.no_push,
            inter_package_delay_ms: config.package_delay * 1000,
            registry: config.registry.clone(),
            allow_dirty: config.allow_dirty,
            ..Default::default()
        };

        let mut release_state = ReleaseState::new(new_version.clone(), bump.clone(), release_config);

        // Initialize state manager in temp clone
        let mut state = create_state_manager_at(&state_path)?;

        // Begin release process
        release_state.add_checkpoint(
            "release_started".to_string(),
            ReleasePhase::Validation,
            None,
            false,
        );
        state.save_state(&release_state).await?;

        // Phase 1: Version Update
        release_state.set_phase(ReleasePhase::VersionUpdate);
        state.save_state(&release_state).await?;

        let version_result = version_manager.release_version(bump)?;
        release_state.set_version_state(&version_result.update_result);
        release_state.add_checkpoint(
            "version_updated".to_string(),
            ReleasePhase::VersionUpdate,
            None,
            true,
        );
        state.save_state(&release_state).await?;

        // Phase 1.5: Sign Artifacts (macOS only)
        let signed_artifacts: Vec<PathBuf> = if cfg!(target_os = "macos") {
            let sign_dir = temp_dir.join("target/release-artifacts");
            std::fs::create_dir_all(&sign_dir)
                .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                    command: "create_artifact_dir".to_string(),
                    reason: e.to_string(),
                }))?;

            match kodegen_bundler_sign::build_and_sign_helper(&sign_dir) {
                Ok(helper_zip) => vec![helper_zip],
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        if !signed_artifacts.is_empty() {
            release_state.add_checkpoint(
                "artifacts_signed".to_string(),
                ReleasePhase::VersionUpdate,
                Some(serde_json::Value::String(format!("{} artifact(s) signed", signed_artifacts.len()))),
                true,
            );
            state.save_state(&release_state).await?;
        }

        // Phase 1.6: Bundle Artifacts (if requested)
        let bundled_artifacts: Vec<crate::bundler::BundledArtifact> = if config.with_bundles || config.upload_bundles {
            create_bundles(&workspace, &new_version).unwrap_or_default()
        } else {
            vec![]
        };

        if !bundled_artifacts.is_empty() {
            release_state.add_checkpoint(
                "artifacts_bundled".to_string(),
                ReleasePhase::VersionUpdate,
                Some(serde_json::Value::String(format!(
                    "{} bundle(s) created: {}",
                    bundled_artifacts.len(),
                    bundled_artifacts.iter()
                        .map(|a| format!("{:?}", a.package_type))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))),
                true,
            );
            state.save_state(&release_state).await?;
        }

        // Phase 2: Git Operations
        release_state.set_phase(ReleasePhase::GitOperations);
        state.save_state(&release_state).await?;

        let git_result = git.perform_release(&new_version, !config.no_push).await?;
        release_state.set_git_state(Some(&git_result.commit), Some(&git_result.tag));

        if let Some(push_info) = &git_result.push_info {
            release_state.set_git_push_state(push_info);
        }

        release_state.add_checkpoint(
            "git_operations_complete".to_string(),
            ReleasePhase::GitOperations,
            None,
            true,
        );
        state.save_state(&release_state).await?;

        // Phase 2.5: GitHub Release (if configured)
        let github_release_url = if config.github_release {
            release_state.set_phase(ReleasePhase::GitHubRelease);
            state.save_state(&release_state).await?;

            if let Ok((owner, repo)) = parse_github_repo(config.github_repo.as_deref()) {
                let release_notes = config.release_notes.as_ref()
                    .and_then(|path| std::fs::read_to_string(path).ok());

                let github_config = crate::github::GitHubReleaseConfig {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    draft: config.github_draft,
                    prerelease_for_zero_versions: true,
                    notes: None,
                    token: None,
                };

                if let Ok(github_manager) = crate::github::GitHubReleaseManager::new(github_config) {
                    let commit_sha = &git_result.commit.hash;

                    if let Ok(github_result) = github_manager.create_release(&new_version, commit_sha, release_notes).await {
                        release_state.set_github_state(owner, repo, Some(&github_result));
                        release_state.add_checkpoint(
                            "github_release_created".to_string(),
                            ReleasePhase::GitHubRelease,
                            None,
                            true,
                        );
                        state.save_state(&release_state).await?;

                        // Upload all artifacts
                        let all_artifacts: Vec<PathBuf> = signed_artifacts.iter()
                            .chain(bundled_artifacts.iter().flat_map(|ba| ba.paths.iter()))
                            .cloned()
                            .collect();

                        if !all_artifacts.is_empty()
                            && let Ok(urls) = github_manager.upload_artifacts(github_result.release_id, &all_artifacts).await {
                                release_state.add_checkpoint(
                                    "artifacts_uploaded".to_string(),
                                    ReleasePhase::GitHubRelease,
                                    Some(serde_json::Value::Object({
                                        let mut map = serde_json::Map::new();
                                        map.insert("count".to_string(),
                                            serde_json::Value::Number(urls.len().into()));
                                        map.insert("signed".to_string(),
                                            serde_json::Value::Number(signed_artifacts.len().into()));
                                        map.insert("bundled".to_string(),
                                            serde_json::Value::Number(bundled_artifacts.len().into()));
                                        map
                                    })),
                                    true,
                                );
                                state.save_state(&release_state).await?;
                            }

                        Some(github_result.html_url)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Phase 3: Publishing
        release_state.set_phase(ReleasePhase::Publishing);

        let publish_order = crate::workspace::DependencyGraph::build(&workspace)?.publish_order()?;
        release_state.init_publish_state(publish_order.tier_count());
        state.save_state(&release_state).await?;

        let publish_result = publisher.publish_all_packages().await?;

        for package_result in publish_result.successful_publishes.values() {
            release_state.add_published_package(package_result);
        }

        for (package_name, error) in &publish_result.failed_packages {
            release_state.add_failed_package(package_name.clone(), error.clone());
        }

        release_state.add_checkpoint(
            "publishing_complete".to_string(),
            ReleasePhase::Publishing,
            None,
            true,
        );
        state.save_state(&release_state).await?;

        // Phase 4: Cleanup
        release_state.set_phase(ReleasePhase::Cleanup);
        state.save_state(&release_state).await?;

        git.clear_release_state();
        publisher.clear_state();

        release_state.set_phase(ReleasePhase::Completed);
        release_state.add_checkpoint(
            "release_completed".to_string(),
            ReleasePhase::Completed,
            None,
            false,
        );
        state.save_state(&release_state).await?;

        state.cleanup_state()?;

        Ok(ReleaseResult {
            version: new_version,
            published_packages: publish_result.successful_publishes.keys().cloned().collect(),
            git_commit: Some(git_result.commit.hash),
            git_tag: Some(git_result.tag.name),
            github_release: github_release_url,
            artifacts_signed: signed_artifacts.len(),
            bundles_created: bundled_artifacts.len(),
        })
    }

    /// Rollback a failed release
    pub async fn rollback(&mut self, force: bool, git_only: bool, packages_only: bool) -> Result<RollbackResult> {
        use crate::git::GitConfig;
        use crate::state::{ReleasePhase, create_state_manager_at};

        // Load release state
        let state_path = PathBuf::from(".cyrup_release_state.json");
        let mut state_manager = create_state_manager_at(&state_path)?;
        let load_result = state_manager.load_state().await?;
        let mut release_state = load_result.state;

        // Validate rollback conditions
        if release_state.current_phase == ReleasePhase::Completed && !force {
            return Err(ReleaseError::State(crate::error::StateError::SaveFailed {
                reason: "Release completed successfully. Use force=true to rollback anyway".to_string(),
            }));
        }

        release_state.set_phase(ReleasePhase::RollingBack);
        state_manager.save_state(&release_state).await?;

        let mut result = RollbackResult {
            packages_yanked: vec![],
            git_reverted: false,
            github_deleted: false,
        };

        // Rollback publishing if needed and not git-only
        if !git_only && release_state.publish_state.is_some() {
            let publisher = Publisher::new(self.workspace.clone())?;
            let rollback_result = publisher.rollback_published_packages().await?;

            result.packages_yanked = rollback_result.yanked_packages.keys().cloned().collect();
        }

        // Rollback git operations if needed and not packages-only
        if !packages_only && release_state.git_state.is_some() {
            let git_config = GitConfig::default();
            let mut git_manager = GitManager::with_config(".", git_config).await?;

            let git_rollback = git_manager.rollback_release().await?;
            result.git_reverted = git_rollback.success;
        }

        // Rollback GitHub release if created
        if let Some(github_state) = &release_state.github_state
            && let Some(release_id) = github_state.release_id {
                let github_token = std::env::var("GH_TOKEN")
                    .or_else(|_| std::env::var("GITHUB_TOKEN"))
                    .ok();

                if let Some(token) = github_token {
                    let github_config = crate::github::GitHubReleaseConfig {
                        owner: github_state.owner.clone(),
                        repo: github_state.repo.clone(),
                        draft: false,
                        prerelease_for_zero_versions: true,
                        notes: None,
                        token: Some(token),
                    };

                    if let Ok(github_manager) = crate::github::GitHubReleaseManager::new(github_config)
                        && let Ok(()) = github_manager.delete_release(release_id).await {
                            result.github_deleted = true;
                        }
                }
            }

        // Cleanup state
        state_manager.cleanup_state()?;

        Ok(result)
    }

    /// Resume an interrupted release
    pub async fn resume(&mut self, config: ReleaseConfig) -> Result<ReleaseResult> {
        use crate::git::GitConfig;
        use crate::publish::PublisherConfig;
        use crate::state::{ReleasePhase, create_state_manager_at};

        // Load release state
        let state_path = PathBuf::from(".cyrup_release_state.json");
        let mut state_manager = create_state_manager_at(&state_path)?;
        let load_result = state_manager.load_state().await?;
        let mut release_state = load_result.state;

        if !release_state.is_resumable() {
            return Err(ReleaseError::State(crate::error::StateError::Corrupted {
                reason: "Release is not in a resumable state".to_string(),
            }));
        }

        let new_version = release_state.target_version.clone();
        let bump = release_state.version_bump.clone();
        let current_phase = release_state.current_phase;

        // Re-initialize managers
        let mut version_manager = VersionManager::new(self.workspace.clone());

        // Resume from current phase
        if current_phase <= ReleasePhase::VersionUpdate
            && release_state.version_state.is_none() {
                release_state.set_phase(ReleasePhase::VersionUpdate);
                state_manager.save_state(&release_state).await?;

                let version_result = version_manager.release_version(bump.clone())?;
                release_state.set_version_state(&version_result.update_result);
                state_manager.save_state(&release_state).await?;
            }

        // Continue with git operations if not completed
        if current_phase <= ReleasePhase::GitOperations
            && release_state.git_state.is_none() {
                release_state.set_phase(ReleasePhase::GitOperations);
                state_manager.save_state(&release_state).await?;

                let git_config = GitConfig {
                    default_remote: "origin".to_string(),
                    annotated_tags: true,
                    auto_push_tags: !config.no_push,
                    ..Default::default()
                };
                let mut git_manager = GitManager::with_config(".", git_config).await?;

                let git_result = git_manager.perform_release(&new_version, !config.no_push).await?;
                release_state.set_git_state(Some(&git_result.commit), Some(&git_result.tag));
                state_manager.save_state(&release_state).await?;
            }

        // Continue with publishing if not completed
        if current_phase <= ReleasePhase::Publishing {
            release_state.set_phase(ReleasePhase::Publishing);
            state_manager.save_state(&release_state).await?;

            let publisher_config = PublisherConfig {
                inter_package_delay: Duration::from_secs(config.package_delay),
                registry: config.registry.clone(),
                max_concurrent_per_tier: 1,
                ..Default::default()
            };
            let mut publisher = Publisher::with_config(self.workspace.clone(), publisher_config)?;

            let publish_result = publisher.publish_all_packages().await?;

            for package_result in publish_result.successful_publishes.values() {
                let already_published = release_state.publish_state.as_ref()
                    .is_some_and(|ps| ps.published_packages.contains_key(&package_result.package_name));
                if !already_published {
                    release_state.add_published_package(package_result);
                }
            }

            state_manager.save_state(&release_state).await?;
        }

        // Mark as completed
        release_state.set_phase(ReleasePhase::Completed);
        state_manager.save_state(&release_state).await?;
        state_manager.cleanup_state()?;

        Ok(ReleaseResult {
            version: new_version,
            published_packages: release_state.publish_state.as_ref()
                .map_or_else(Vec::new, |ps| ps.published_packages.keys().cloned().collect()),
            git_commit: release_state.git_state.as_ref()
                .and_then(|g| g.release_commit.as_ref()).map(|c| c.hash.clone()),
            git_tag: release_state.git_state.as_ref()
                .and_then(|g| g.release_tag.as_ref()).map(|t| t.name.clone()),
            github_release: release_state.github_state.as_ref()
                .and_then(|g| g.html_url.as_ref()).cloned(),
            artifacts_signed: 0,
            bundles_created: 0,
        })
    }

    /// Get the workspace information
    pub fn workspace(&self) -> &WorkspaceInfo {
        &self.workspace
    }

    /// Get the git manager
    pub fn git(&mut self) -> &mut GitManager {
        &mut self.git
    }

    /// Get the publisher
    pub fn publisher(&mut self) -> &mut Publisher {
        &mut self.publisher
    }

    /// Get the state manager
    pub fn state(&mut self) -> &mut StateManager {
        &mut self.state
    }
}
*/
// END LEGACY CODE - ReleaseManager implementation

// Legacy helper functions (unused - real implementations in cli/commands/helpers.rs)
#[allow(dead_code)]
fn create_bundles(
    workspace: &workspace::WorkspaceInfo,
    version: &semver::Version,
) -> Result<Vec<crate::bundler::BundledArtifact>> {
    use crate::bundler::{
        BundleBinary, BundleSettings, Bundler, DebianSettings, PackageSettings, RpmSettings,
        SettingsBuilder,
    };
    use crate::error::CliError;
    use std::path::PathBuf;

    // Extract product name from first package
    let product_name = workspace
        .packages
        .values()
        .next()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "app".to_string());

    // Extract description from workspace config
    let description = workspace
        .workspace_config
        .package
        .as_ref()
        .and_then(|p| p.other.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or("Rust application")
        .to_string();

    // Build package settings with workspace metadata
    let package_settings = PackageSettings {
        product_name,
        version: version.to_string(),
        description,
        ..Default::default()
    };

    // Discover binaries from workspace
    let binaries = workspace
        .packages
        .values()
        .filter(|pkg| {
            // Check if package has binary targets
            let manifest = std::fs::read_to_string(&pkg.cargo_toml_path).ok();
            manifest
                .map(|m| m.contains("[[bin]]") || (m.contains("name = ") && !m.contains("[lib]")))
                .unwrap_or(false)
        })
        .map(|pkg| {
            let is_main = pkg.name == "kodegen_install";
            BundleBinary::new(pkg.name.clone(), is_main)
        })
        .collect::<Vec<_>>();

    if binaries.is_empty() {
        return Err(ReleaseError::Cli(CliError::InvalidArguments {
            reason: "No binary crates found in workspace".to_string(),
        }));
    }

    // Configure platform-specific maintainer scripts
    let bundle_settings = BundleSettings {
        deb: DebianSettings {
            post_install_script: Some(PathBuf::from("packages/bundler-release/postinst.deb.sh")),
            ..Default::default()
        },
        rpm: RpmSettings {
            post_install_script: Some(PathBuf::from("packages/bundler-release/postinst.rpm.sh")),
            ..Default::default()
        },
        ..Default::default()
    };

    // Validate that all binaries exist before bundling
    let binary_dir = workspace.root.join("target/release");
    for binary in &binaries {
        let binary_path = binary_dir.join(binary.name());
        let binary_path_exe = binary_dir.join(format!("{}.exe", binary.name()));

        if !binary_path.exists() && !binary_path_exe.exists() {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "validate_binaries".to_string(),
                reason: format!(
                    "Binary '{}' not found in {}\n\
                     Build all binaries first:\n  \
                     cargo build --release --bin kodegen_install\n  \
                     cargo build --release --bin kodegen\n  \
                     cargo build --release --bin kodegend",
                    binary.name(),
                    binary_dir.display()
                ),
            }));
        }
    }

    // Use SettingsBuilder to create Settings
    let settings = SettingsBuilder::new()
        .project_out_directory(workspace.root.join("target/release"))
        .package_settings(package_settings)
        .binaries(binaries)
        .bundle_settings(bundle_settings)
        .build()
        .map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: "build_settings".to_string(),
                reason: e.to_string(),
            })
        })?;

    // Create bundler with Settings
    let bundler = Bundler::new(settings).map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "create_bundler".to_string(),
            reason: e.to_string(),
        })
    })?;

    bundler.bundle().map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "bundle_artifacts".to_string(),
            reason: e.to_string(),
        })
    })
}

#[allow(dead_code)]
fn parse_github_repo(repo_str: Option<&str>) -> Result<(String, String)> {
    use crate::error::CliError;

    if let Some(repo) = repo_str {
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() == 2 {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(ReleaseError::Cli(CliError::InvalidArguments {
                reason: "GitHub repo must be in format 'owner/repo'".to_string(),
            }))
        }
    } else {
        // Try to detect from git remote
        Err(ReleaseError::Cli(CliError::InvalidArguments {
            reason: "GitHub repo not specified".to_string(),
        }))
    }
}
