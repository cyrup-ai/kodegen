//! Release state tracking and serialization.
//!
//! This module provides comprehensive state tracking for release operations,
//! enabling resume capabilities and rollback coordination.

use crate::error::{Result, StateError};
use crate::git::{CommitInfo, PushInfo, TagInfo};
use crate::publish::PublishResult;
use crate::version::{UpdateResult, VersionBump};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Current version of the state format (for migration)
pub const STATE_FORMAT_VERSION: u32 = 1;

/// Complete release operation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseState {
    /// Version of the state format
    pub format_version: u32,
    /// Unique ID for this release operation
    pub release_id: String,
    /// Version being released
    pub target_version: semver::Version,
    /// Type of version bump
    pub version_bump: VersionBump,
    /// Timestamp when release started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when release was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Current phase of the release
    pub current_phase: ReleasePhase,
    /// Checkpoints passed during release
    pub checkpoints: Vec<ReleaseCheckpoint>,
    /// Version management state
    pub version_state: Option<VersionState>,
    /// Git operation state
    pub git_state: Option<GitState>,
    /// GitHub release state
    pub github_state: Option<GitHubState>,
    /// Publishing state
    pub publish_state: Option<PublishState>,
    /// Any errors encountered during release
    pub errors: Vec<ReleaseError>,
    /// Release configuration
    pub config: ReleaseConfig,
}

/// Phase of the release operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReleasePhase {
    /// Initial validation and preparation
    Validation,
    /// Version updating and file modifications
    VersionUpdate,
    /// Git operations (commit, tag)
    GitOperations,
    /// GitHub release creation
    GitHubRelease,
    /// Package publishing
    Publishing,
    /// Post-release cleanup
    Cleanup,
    /// Release completed successfully
    Completed,
    /// Release failed and needs rollback
    Failed,
    /// Rollback in progress
    RollingBack,
    /// Rollback completed
    RolledBack,
}

/// Checkpoint in the release process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCheckpoint {
    /// Checkpoint name
    pub name: String,
    /// Phase this checkpoint belongs to
    pub phase: ReleasePhase,
    /// Timestamp when checkpoint was reached
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Any data associated with this checkpoint
    pub data: Option<serde_json::Value>,
    /// Whether this checkpoint can be rolled back
    pub rollback_capable: bool,
}

/// Version management state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionState {
    /// Previous version before update
    pub previous_version: semver::Version,
    /// New version after update
    pub new_version: semver::Version,
    /// Result of version update operation
    pub update_result: Option<VersionUpdateInfo>,
    /// Files that were modified during version update
    pub modified_files: Vec<PathBuf>,
    /// Backup locations for rollback
    pub backup_files: Vec<FileBackup>,
}

/// Git operation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    /// Previous HEAD commit before release
    pub previous_head: Option<String>,
    /// Commit created for this release
    pub release_commit: Option<GitCommitInfo>,
    /// Tag created for this release
    pub release_tag: Option<GitTagInfo>,
    /// Push information
    pub push_info: Option<GitPushInfo>,
    /// Whether git operations have been pushed to remote
    pub pushed_to_remote: bool,
}

/// GitHub release state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubState {
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Release ID
    pub release_id: Option<u64>,
    /// Release URL
    pub html_url: Option<String>,
    /// Whether this was a draft
    pub draft: bool,
    /// Whether this was a prerelease
    pub prerelease: bool,
}

/// Publishing state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishState {
    /// Packages that have been successfully published
    pub published_packages: HashMap<String, PublishPackageInfo>,
    /// Packages that failed to publish
    pub failed_packages: HashMap<String, String>,
    /// Current tier being published
    pub current_tier: usize,
    /// Total tiers to publish
    pub total_tiers: usize,
    /// Publishing start time
    pub publishing_started_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Error encountered during release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseError {
    /// Error message
    pub message: String,
    /// Phase where error occurred
    pub phase: ReleasePhase,
    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether this error is recoverable
    pub recoverable: bool,
    /// Stack trace or additional context
    pub context: Option<String>,
}

/// Release configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseConfig {
    /// Whether to perform dry run first
    pub dry_run_first: bool,
    /// Whether to push to remote
    pub push_to_remote: bool,
    /// Inter-package delay in milliseconds
    pub inter_package_delay_ms: u64,
    /// Registry to publish to
    pub registry: Option<String>,
    /// Whether to allow dirty working directory
    pub allow_dirty: bool,
    /// Additional configuration options
    pub additional_options: HashMap<String, serde_json::Value>,
}

/// Simplified version update information for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionUpdateInfo {
    /// Number of packages updated
    pub packages_updated: usize,
    /// Number of dependencies updated
    pub dependencies_updated: usize,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Git commit information for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommitInfo {
    /// Commit hash
    pub hash: String,
    /// Short commit hash
    pub short_hash: String,
    /// Commit message
    pub message: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Git tag information for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTagInfo {
    /// Tag name
    pub name: String,
    /// Tag message
    pub message: Option<String>,
    /// Target commit hash
    pub target_commit: String,
    /// Tag timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether this is an annotated tag
    pub is_annotated: bool,
}

/// Git push information for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitPushInfo {
    /// Remote name
    pub remote_name: String,
    /// Number of commits pushed
    pub commits_pushed: usize,
    /// Number of tags pushed
    pub tags_pushed: usize,
    /// Any warnings
    pub warnings: Vec<String>,
}

/// Information about a published package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishPackageInfo {
    /// Package name
    pub package_name: String,
    /// Version published
    pub version: semver::Version,
    /// Duration of publish operation in milliseconds
    pub duration_ms: u64,
    /// Number of retry attempts
    pub retry_attempts: usize,
    /// Warnings from publish
    pub warnings: Vec<String>,
    /// Timestamp when published
    pub published_at: chrono::DateTime<chrono::Utc>,
}

/// File backup information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBackup {
    /// Original file path
    pub file_path: PathBuf,
    /// Backup content
    pub backup_content: String,
    /// Timestamp of backup
    pub backup_timestamp: chrono::DateTime<chrono::Utc>,
}

impl ReleaseState {
    /// Create a new release state
    pub fn new(
        target_version: semver::Version,
        version_bump: VersionBump,
        config: ReleaseConfig,
    ) -> Self {
        let now = chrono::Utc::now();
        let release_id = format!("release-{}-{}", target_version, now.timestamp());

        Self {
            format_version: STATE_FORMAT_VERSION,
            release_id,
            target_version,
            version_bump,
            started_at: now,
            updated_at: now,
            current_phase: ReleasePhase::Validation,
            checkpoints: Vec::new(),
            version_state: None,
            git_state: None,
            github_state: None,
            publish_state: None,
            errors: Vec::new(),
            config,
        }
    }

    /// Add a checkpoint to the release state
    pub fn add_checkpoint(
        &mut self,
        name: String,
        phase: ReleasePhase,
        data: Option<serde_json::Value>,
        rollback_capable: bool,
    ) {
        let checkpoint = ReleaseCheckpoint {
            name,
            phase,
            timestamp: chrono::Utc::now(),
            data,
            rollback_capable,
        };

        self.checkpoints.push(checkpoint);
        self.updated_at = chrono::Utc::now();
    }

    /// Set current phase
    pub fn set_phase(&mut self, phase: ReleasePhase) {
        self.current_phase = phase;
        self.updated_at = chrono::Utc::now();
    }

    /// Add an error to the release state
    pub fn add_error(
        &mut self,
        message: String,
        phase: ReleasePhase,
        recoverable: bool,
        context: Option<String>,
    ) {
        let error = ReleaseError {
            message,
            phase,
            timestamp: chrono::Utc::now(),
            recoverable,
            context,
        };

        self.errors.push(error);
        self.updated_at = chrono::Utc::now();
    }

    /// Update version state
    pub fn set_version_state(&mut self, update_result: &UpdateResult) {
        self.version_state = Some(VersionState {
            previous_version: update_result.previous_version.clone(),
            new_version: update_result.new_version.clone(),
            update_result: Some(VersionUpdateInfo {
                packages_updated: update_result.packages_updated,
                dependencies_updated: update_result.dependencies_updated,
                duration_ms: 0, // This would need to be tracked separately
            }),
            modified_files: update_result.modified_files.clone(),
            backup_files: Vec::new(), // This would be populated by the version manager
        });

        self.updated_at = chrono::Utc::now();
    }

    /// Update git state
    pub fn set_git_state(&mut self, commit: Option<&CommitInfo>, tag: Option<&TagInfo>) {
        if self.git_state.is_none() {
            self.git_state = Some(GitState {
                previous_head: None,
                release_commit: None,
                release_tag: None,
                push_info: None,
                pushed_to_remote: false,
            });
        }

        if let Some(git_state) = &mut self.git_state {
            if let Some(commit) = commit {
                git_state.release_commit = Some(GitCommitInfo {
                    hash: commit.hash.clone(),
                    short_hash: commit.short_hash.clone(),
                    message: commit.message.clone(),
                    author_name: commit.author_name.clone(),
                    author_email: commit.author_email.clone(),
                    timestamp: commit.timestamp,
                });
            }

            if let Some(tag) = tag {
                git_state.release_tag = Some(GitTagInfo {
                    name: tag.name.clone(),
                    message: tag.message.clone(),
                    target_commit: tag.target_commit.clone(),
                    timestamp: tag.timestamp,
                    is_annotated: tag.is_annotated,
                });
            }
        }

        self.updated_at = chrono::Utc::now();
    }

    /// Update git push state
    pub fn set_git_push_state(&mut self, push_info: &PushInfo) {
        if let Some(git_state) = &mut self.git_state {
            git_state.push_info = Some(GitPushInfo {
                remote_name: push_info.remote_name.clone(),
                commits_pushed: push_info.commits_pushed,
                tags_pushed: push_info.tags_pushed,
                warnings: push_info.warnings.clone(),
            });
            git_state.pushed_to_remote = true;
        }

        self.updated_at = chrono::Utc::now();
    }

    /// Update GitHub release state
    pub fn set_github_state(
        &mut self,
        owner: String,
        repo: String,
        result: Option<&crate::github::GitHubReleaseResult>,
    ) {
        if let Some(result) = result {
            self.github_state = Some(GitHubState {
                owner,
                repo,
                release_id: Some(result.release_id),
                html_url: Some(result.html_url.clone()),
                draft: result.draft,
                prerelease: result.prerelease,
            });
        }
        self.updated_at = chrono::Utc::now();
    }

    /// Initialize publishing state
    pub fn init_publish_state(&mut self, total_tiers: usize) {
        self.publish_state = Some(PublishState {
            published_packages: HashMap::new(),
            failed_packages: HashMap::new(),
            current_tier: 0,
            total_tiers,
            publishing_started_at: Some(chrono::Utc::now()),
        });

        self.updated_at = chrono::Utc::now();
    }

    /// Add published package
    pub fn add_published_package(&mut self, publish_result: &PublishResult) {
        if let Some(publish_state) = &mut self.publish_state {
            let package_info = PublishPackageInfo {
                package_name: publish_result.package_name.clone(),
                version: publish_result.version.clone(),
                duration_ms: publish_result.duration.as_millis() as u64,
                retry_attempts: publish_result.retry_attempts,
                warnings: publish_result.warnings.clone(),
                published_at: chrono::Utc::now(),
            };

            publish_state
                .published_packages
                .insert(publish_result.package_name.clone(), package_info);
        }

        self.updated_at = chrono::Utc::now();
    }

    /// Add failed package
    pub fn add_failed_package(&mut self, package_name: String, error: String) {
        if let Some(publish_state) = &mut self.publish_state {
            publish_state.failed_packages.insert(package_name, error);
        }

        self.updated_at = chrono::Utc::now();
    }

    /// Set current publishing tier
    pub fn set_current_tier(&mut self, tier: usize) {
        if let Some(publish_state) = &mut self.publish_state {
            publish_state.current_tier = tier;
        }

        self.updated_at = chrono::Utc::now();
    }

    /// Check if release is resumable
    pub fn is_resumable(&self) -> bool {
        matches!(
            self.current_phase,
            ReleasePhase::Validation
                | ReleasePhase::VersionUpdate
                | ReleasePhase::GitOperations
                | ReleasePhase::Publishing
        ) && !self.has_critical_errors()
    }

    /// Check if release has critical errors
    pub fn has_critical_errors(&self) -> bool {
        self.errors.iter().any(|e| !e.recoverable)
    }

    /// Get rollback checkpoints (in reverse order)
    pub fn get_rollback_checkpoints(&self) -> Vec<&ReleaseCheckpoint> {
        self.checkpoints
            .iter()
            .filter(|cp| cp.rollback_capable)
            .rev()
            .collect()
    }

    /// Get progress percentage
    pub fn progress_percentage(&self) -> f64 {
        match self.current_phase {
            ReleasePhase::Validation => 10.0,
            ReleasePhase::VersionUpdate => 30.0,
            ReleasePhase::GitOperations => 50.0,
            ReleasePhase::Publishing => {
                if let Some(publish_state) = &self.publish_state {
                    if publish_state.total_tiers > 0 {
                        let tier_progress = (publish_state.current_tier as f64
                            / publish_state.total_tiers as f64)
                            * 40.0;
                        50.0 + tier_progress
                    } else {
                        70.0
                    }
                } else {
                    70.0
                }
            }
            ReleasePhase::GitHubRelease => 85.0,
            ReleasePhase::Cleanup => 95.0,
            ReleasePhase::Completed => 100.0,
            ReleasePhase::Failed | ReleasePhase::RollingBack | ReleasePhase::RolledBack => {
                // Progress doesn't apply to failure states
                0.0
            }
        }
    }

    /// Get elapsed time
    pub fn elapsed_time(&self) -> chrono::Duration {
        self.updated_at - self.started_at
    }

    /// Validate state consistency
    pub fn validate(&self) -> Result<()> {
        // Check format version
        if self.format_version != STATE_FORMAT_VERSION {
            return Err(StateError::VersionMismatch {
                expected: STATE_FORMAT_VERSION.to_string(),
                found: self.format_version.to_string(),
            }
            .into());
        }

        // Phase can exist without corresponding state - this represents work in progress
        // The resume function handles incomplete phases by checking for missing state

        Ok(())
    }

    /// Create a summary of the release state
    pub fn summary(&self) -> String {
        let elapsed = self.elapsed_time();
        let progress = self.progress_percentage();

        format!(
            "Release v{} ({:?}) - {:.1}% complete - {} elapsed",
            self.target_version,
            self.current_phase,
            progress,
            format_duration(elapsed)
        )
    }
}

/// Format duration for display
fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            dry_run_first: true,
            push_to_remote: true,
            inter_package_delay_ms: 15000, // 15 seconds as requested
            registry: None,
            allow_dirty: false,
            additional_options: HashMap::new(),
        }
    }
}

impl std::fmt::Display for ReleasePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleasePhase::Validation => write!(f, "Validation"),
            ReleasePhase::VersionUpdate => write!(f, "Version Update"),
            ReleasePhase::GitOperations => write!(f, "Git Operations"),
            ReleasePhase::Publishing => write!(f, "Publishing"),
            ReleasePhase::GitHubRelease => write!(f, "GitHub Release"),
            ReleasePhase::Cleanup => write!(f, "Cleanup"),
            ReleasePhase::Completed => write!(f, "Completed"),
            ReleasePhase::Failed => write!(f, "Failed"),
            ReleasePhase::RollingBack => write!(f, "Rolling Back"),
            ReleasePhase::RolledBack => write!(f, "Rolled Back"),
        }
    }
}
