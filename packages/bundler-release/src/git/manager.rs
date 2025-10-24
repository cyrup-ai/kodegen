//! Git manager for coordinating release operations.
//!
//! This module provides high-level Git management for release workflows,
//! coordinating commits, tags, pushes, and rollbacks.

use crate::error::{GitError, Result};
use crate::git::{
    BranchInfo, CommitInfo, GitOperations, KodegenGitOperations, PushInfo, TagInfo,
    ValidationResult,
};
use semver::Version;
use std::path::Path;

/// High-level Git manager for release operations
#[derive(Debug)]
pub struct GitManager {
    /// Underlying Git repository
    repository: KodegenGitOperations,
    /// Configuration for Git operations
    config: GitConfig,
    /// Release state tracking
    release_state: ReleaseState,
}

/// Configuration for Git operations
#[derive(Debug, Clone)]
pub struct GitConfig {
    /// Default remote name for push operations
    pub default_remote: String,
    /// Whether to create annotated tags
    pub annotated_tags: bool,
    /// Whether to push tags automatically
    pub auto_push_tags: bool,
    /// Custom commit message template
    pub commit_message_template: Option<String>,
    /// Custom tag message template
    pub tag_message_template: Option<String>,
    /// Whether to verify signatures
    pub verify_signatures: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            default_remote: "origin".to_string(),
            annotated_tags: true,
            auto_push_tags: true,
            commit_message_template: None,
            tag_message_template: None,
            verify_signatures: false,
        }
    }
}

/// State tracking for release operations
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ReleaseState {
    /// Commit created for this release
    release_commit: Option<CommitInfo>,
    /// Tag created for this release
    release_tag: Option<TagInfo>,
    /// Whether commits have been pushed
    commits_pushed: bool,
    /// Whether tags have been pushed
    tags_pushed: bool,
    /// Previous HEAD before release (for rollback)
    previous_head: Option<String>,
}

/// Result of a complete release operation
#[derive(Debug, Clone)]
pub struct ReleaseResult {
    /// Version that was released
    pub version: Version,
    /// Commit information
    pub commit: CommitInfo,
    /// Tag information
    pub tag: TagInfo,
    /// Push information for main branch (if pushed)
    pub push_info: Option<PushInfo>,
    /// Release branch information
    pub release_branch: BranchInfo,
    /// Push information for release branch (if pushed)
    pub release_branch_push_info: Option<PushInfo>,
    /// Duration of the operation
    pub duration: std::time::Duration,
}

/// Result of a rollback operation
#[derive(Debug, Clone)]
pub struct RollbackResult {
    /// Whether rollback was successful
    pub success: bool,
    /// Operations that were rolled back
    pub rolled_back_operations: Vec<String>,
    /// Any warnings during rollback
    pub warnings: Vec<String>,
    /// Duration of the rollback
    pub duration: std::time::Duration,
}

impl GitManager {
    /// Create a new Git manager for the given repository path
    pub async fn new<P: AsRef<Path>>(repo_path: P) -> Result<Self> {
        let repository = KodegenGitOperations::open(repo_path).await?;
        let config = GitConfig::default();
        let release_state = ReleaseState::default();

        Ok(Self {
            repository,
            config,
            release_state,
        })
    }

    /// Create a Git manager with custom configuration
    pub async fn with_config<P: AsRef<Path>>(repo_path: P, config: GitConfig) -> Result<Self> {
        let repository = KodegenGitOperations::open(repo_path).await?;
        let release_state = ReleaseState::default();

        Ok(Self {
            repository,
            config,
            release_state,
        })
    }

    /// Perform a complete automated release operation
    ///
    /// This method handles the full release workflow from ANY starting state:
    /// 1. Commits any uncommitted work on the current branch (never leaves work dirty)
    /// 2. Switches to main branch
    /// 3. Merges work from the original branch (ONLY if not a release branch)
    /// 4. Creates a new release branch matching the bumped version
    /// 5. Commits version changes on the release branch
    /// 6. Creates version tag
    /// 7. Pushes release branch with tags to remote (if requested)
    /// 8. ALWAYS returns to main branch (using checkout, never reset)
    pub async fn perform_release(
        &mut self,
        version: &Version,
        push_to_remote: bool,
    ) -> Result<ReleaseResult> {
        let start_time = std::time::Instant::now();

        // Step 1: Save starting state - record current branch name
        let original_branch = self.repository.get_current_branch().await?;
        let original_branch_name = original_branch.name.clone();

        // Step 2: Commit work-in-progress if working directory is dirty
        // This ensures we NEVER have uncommitted changes that could be lost
        let is_clean = self.repository.is_working_directory_clean().await?;
        if !is_clean {
            match self
                .repository
                .commit_all_changes("WIP: Auto-commit before release")
                .await
            {
                Ok(_wip_commit) => {
                    // WIP committed successfully
                }
                Err(e) => {
                    // Failed to commit WIP - return to main and error
                    let _ = self.repository.checkout_branch("main").await;
                    return Err(GitError::CommitFailed {
                        reason: format!("Failed to commit work-in-progress: {}", e),
                    }
                    .into());
                }
            }
        }

        // Step 3: Switch to main branch
        if let Err(e) = self.repository.checkout_branch("main").await {
            let _ = self.repository.checkout_branch("main").await;
            return Err(GitError::BranchOperationFailed {
                reason: format!("Failed to checkout main branch: {}", e),
            }
            .into());
        }

        // Step 4: Merge work from original branch (ONLY if not already on main AND not a release branch)
        // Release branches (starting with 'v') should NOT be merged back into main
        let is_release_branch = original_branch_name.starts_with('v');
        if original_branch_name != "main"
            && !is_release_branch
            && let Err(e) = self.repository.merge_branch(&original_branch_name).await
        {
            let _ = self.repository.checkout_branch("main").await;
            return Err(GitError::BranchOperationFailed {
                reason: format!(
                    "Failed to merge branch '{}' into main: {}",
                    original_branch_name, e
                ),
            }
            .into());
        }

        // Step 5: Create release branch matching the bumped version
        let release_branch = match self.repository.create_release_branch(version).await {
            Ok(branch) => branch,
            Err(e) => {
                let _ = self.repository.checkout_branch("main").await;
                return Err(GitError::BranchOperationFailed {
                    reason: format!("Failed to create release branch: {}", e),
                }
                .into());
            }
        };

        // Step 6: Commit version changes on release branch
        let commit_message = self.generate_commit_message(version);
        let commit = match self
            .repository
            .create_release_commit(version, Some(commit_message))
            .await
        {
            Ok(c) => c,
            Err(e) => {
                let _ = self.repository.checkout_branch("main").await;
                return Err(GitError::CommitFailed {
                    reason: format!("Failed to create release commit: {}", e),
                }
                .into());
            }
        };
        self.release_state.release_commit = Some(commit.clone());

        // Step 7: Create version tag on release branch
        let tag_message = self.generate_tag_message(version);
        let tag = match self
            .repository
            .create_version_tag(version, Some(tag_message))
            .await
        {
            Ok(t) => t,
            Err(e) => {
                let _ = self.repository.checkout_branch("main").await;
                return Err(GitError::CommitFailed {
                    reason: format!("Failed to create version tag: {}", e),
                }
                .into());
            }
        };
        self.release_state.release_tag = Some(tag.clone());

        // Step 8: Push release branch with tags to remote (if requested)
        let release_branch_push_info = if push_to_remote {
            let branch_name = format!("v{}", version);
            match self
                .repository
                .push_branch_with_tags(&branch_name, Some(&self.config.default_remote))
                .await
            {
                Ok(push_info) => {
                    self.release_state.tags_pushed = true;
                    Some(push_info)
                }
                Err(e) => {
                    let _ = self.repository.checkout_branch("main").await;
                    return Err(GitError::PushFailed {
                        reason: format!(
                            "Failed to push release branch: {}. Local changes preserved.",
                            e
                        ),
                    }
                    .into());
                }
            }
        } else {
            None
        };

        // Step 9: ALWAYS return to main branch
        self.repository.checkout_branch("main").await?;

        let duration = start_time.elapsed();

        Ok(ReleaseResult {
            version: version.clone(),
            commit,
            tag,
            push_info: None, // No longer pushing main branch separately
            release_branch,
            release_branch_push_info,
            duration,
        })
    }

    /// Push main branch to remote (without tags)
    #[allow(dead_code)]
    async fn push_main_branch(&self) -> Result<PushInfo> {
        self.repository
            .push_to_remote(
                Some(&self.config.default_remote),
                false, // Don't push tags yet - tags will be pushed with release branch
            )
            .await
    }

    /// Rollback a release operation
    ///
    /// SAFE ROLLBACK: Only deletes tags, never uses git reset
    /// All work is preserved in commits - nothing is destroyed
    pub async fn rollback_release(&mut self) -> Result<RollbackResult> {
        let start_time = std::time::Instant::now();
        let mut rolled_back_operations = Vec::new();
        let mut warnings = Vec::new();
        let mut success = true;

        // Rollback in reverse order of operations

        // 1. Delete remote tag if it was pushed
        if self.release_state.tags_pushed
            && let Some(ref tag_info) = self.release_state.release_tag
        {
            match self.repository.delete_tag(&tag_info.name, true).await {
                Ok(()) => {
                    rolled_back_operations.push(format!("Deleted remote tag {}", tag_info.name));
                }
                Err(e) => {
                    warnings.push(format!(
                        "Failed to delete remote tag {}: {}",
                        tag_info.name, e
                    ));
                }
            }
        }

        // 2. Delete local tag
        if let Some(ref tag_info) = self.release_state.release_tag {
            match self.repository.delete_tag(&tag_info.name, false).await {
                Ok(()) => {
                    rolled_back_operations.push(format!("Deleted local tag {}", tag_info.name));
                }
                Err(e) => {
                    warnings.push(format!(
                        "Failed to delete local tag {}: {}",
                        tag_info.name, e
                    ));
                    success = false;
                }
            }
        }

        // 3. Return to main branch (safe - no reset, just checkout)
        match self.repository.checkout_branch("main").await {
            Ok(()) => {
                rolled_back_operations.push("Returned to main branch".to_string());
            }
            Err(e) => {
                warnings.push(format!("Failed to checkout main branch: {}", e));
            }
        }

        // Clear release state
        self.release_state = ReleaseState::default();

        let duration = start_time.elapsed();

        Ok(RollbackResult {
            success,
            rolled_back_operations,
            warnings,
            duration,
        })
    }

    /// Validate repository is ready for release
    #[allow(dead_code)]
    async fn validate_for_release(&self) -> Result<()> {
        let validation = self.repository.validate_release_readiness().await?;

        if !validation.is_ready {
            return Err(GitError::DirtyWorkingDirectory.into());
        }

        Ok(())
    }

    /// Generate commit message for release
    fn generate_commit_message(&self, version: &Version) -> String {
        if let Some(ref template) = self.config.commit_message_template {
            template.replace("{version}", &version.to_string())
        } else {
            format!("release: v{}", version)
        }
    }

    /// Generate tag message for release
    fn generate_tag_message(&self, version: &Version) -> String {
        if let Some(ref template) = self.config.tag_message_template {
            template.replace("{version}", &version.to_string())
        } else {
            format!("Release v{}", version)
        }
    }

    /// Check if working directory is clean
    pub async fn is_clean(&self) -> Result<bool> {
        self.repository.is_working_directory_clean().await
    }

    /// Get current branch information
    pub async fn current_branch(&self) -> Result<crate::git::BranchInfo> {
        self.repository.get_current_branch().await
    }

    /// Get recent commit history
    pub async fn recent_commits(&self, count: usize) -> Result<Vec<CommitInfo>> {
        self.repository.get_recent_commits(count).await
    }

    /// Check if a version tag already exists
    pub async fn version_tag_exists(&self, version: &Version) -> Result<bool> {
        let tag_name = format!("v{}", version);
        self.repository.tag_exists(&tag_name).await
    }

    /// Get remote information
    pub async fn remotes(&self) -> Result<Vec<crate::git::RemoteInfo>> {
        self.repository.get_remotes().await
    }

    /// Validate repository for release operations
    pub async fn validate(&self) -> Result<ValidationResult> {
        self.repository.validate_release_readiness().await
    }

    /// Get configuration
    pub fn config(&self) -> &GitConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: GitConfig) {
        self.config = config;
    }

    /// Check if there's an active release
    pub fn has_active_release(&self) -> bool {
        self.release_state.release_commit.is_some() || self.release_state.release_tag.is_some()
    }

    /// Get current release state
    pub fn release_state(&self) -> &ReleaseState {
        &self.release_state
    }

    /// Clear release state (call after successful completion)
    pub fn clear_release_state(&mut self) {
        self.release_state = ReleaseState::default();
    }

    /// Create a backup point before starting operations
    pub async fn create_backup_point(&mut self) -> Result<BackupPoint> {
        let current_branch = self.repository.get_current_branch().await?;
        let recent_commits = self.repository.get_recent_commits(5).await?;

        Ok(BackupPoint {
            branch_name: current_branch.name,
            commit_hash: current_branch.commit_hash,
            timestamp: chrono::Utc::now(),
            recent_commits,
        })
    }

    /// Restore from a backup point
    ///
    /// SAFE RESTORE: Checks out the branch instead of using destructive reset
    /// All work is preserved - this just switches to the backup branch
    pub async fn restore_from_backup(&self, backup: &BackupPoint) -> Result<()> {
        // Safe: Just checkout the branch, don't reset
        self.repository.checkout_branch(&backup.branch_name).await
    }

    /// Get Git repository statistics
    pub async fn get_repository_stats(&self) -> Result<RepositoryStats> {
        let current_branch = self.repository.get_current_branch().await?;
        let is_clean = self.repository.is_working_directory_clean().await?;
        let remotes = self.repository.get_remotes().await?;
        let recent_commits = self.repository.get_recent_commits(10).await?;

        Ok(RepositoryStats {
            current_branch: current_branch.name,
            is_clean,
            remote_count: remotes.len(),
            recent_commit_count: recent_commits.len(),
            has_upstream: current_branch.upstream.is_some(),
        })
    }
}

/// Backup point for repository state
#[derive(Debug, Clone)]
pub struct BackupPoint {
    /// Branch name at backup time
    pub branch_name: String,
    /// Commit hash at backup time
    pub commit_hash: String,
    /// Timestamp when backup was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Recent commits for reference
    pub recent_commits: Vec<CommitInfo>,
}

/// Repository statistics
#[derive(Debug, Clone)]
pub struct RepositoryStats {
    /// Current branch name
    pub current_branch: String,
    /// Whether working directory is clean
    pub is_clean: bool,
    /// Number of configured remotes
    pub remote_count: usize,
    /// Number of recent commits
    pub recent_commit_count: usize,
    /// Whether current branch has upstream
    pub has_upstream: bool,
}

impl ReleaseResult {
    /// Format result for display
    pub fn format_result(&self) -> String {
        let mut result = format!("🎉 Release v{} completed successfully!\n", self.version);
        result.push_str(&format!(
            "📦 Commit: {} ({})\n",
            self.commit.short_hash, self.commit.message
        ));
        result.push_str(&format!(
            "🌿 Release Branch: {}\n",
            self.release_branch.name
        ));
        result.push_str(&format!(
            "🏷️  Tag: {} (on branch {})\n",
            self.tag.name, self.release_branch.name
        ));

        if let Some(ref push_info) = self.push_info {
            result.push_str(&format!(
                "📤 Pushed main to {}: {} commits\n",
                push_info.remote_name, push_info.commits_pushed
            ));
        }

        if let Some(ref branch_push_info) = self.release_branch_push_info {
            result.push_str(&format!(
                "📤 Pushed {} to {}: {} commits, {} tags\n",
                self.release_branch.name,
                branch_push_info.remote_name,
                branch_push_info.commits_pushed,
                branch_push_info.tags_pushed
            ));
        }

        result.push_str(&format!(
            "⏱️  Duration: {:.2}s\n",
            self.duration.as_secs_f64()
        ));

        result
    }
}

impl RollbackResult {
    /// Format rollback result for display
    pub fn format_result(&self) -> String {
        let status = if self.success { "✅" } else { "⚠️" };
        let mut result = format!("{} Rollback completed\n", status);

        if !self.rolled_back_operations.is_empty() {
            result.push_str("🔄 Operations rolled back:\n");
            for op in &self.rolled_back_operations {
                result.push_str(&format!("  - {}\n", op));
            }
        }

        if !self.warnings.is_empty() {
            result.push_str("⚠️  Warnings:\n");
            for warning in &self.warnings {
                result.push_str(&format!("  - {}\n", warning));
            }
        }

        result.push_str(&format!(
            "⏱️  Duration: {:.2}s\n",
            self.duration.as_secs_f64()
        ));

        result
    }
}

impl RepositoryStats {
    /// Format stats for display
    pub fn format_stats(&self) -> String {
        let clean_status = if self.is_clean {
            "✅ Clean"
        } else {
            "❌ Dirty"
        };
        let upstream_status = if self.has_upstream {
            "✅ Has upstream"
        } else {
            "⚠️ No upstream"
        };

        format!(
            "📊 Repository Stats:\n\
             Branch: {} ({})\n\
             Working Directory: {}\n\
             Remotes: {}\n\
             Recent Commits: {}\n\
             Upstream: {}",
            self.current_branch,
            clean_status,
            clean_status,
            self.remote_count,
            self.recent_commit_count,
            upstream_status
        )
    }
}
