//! Core Git operations trait and types for release management.
//!
//! This module defines the GitOperations trait that specifies all Git operations
//! needed for release workflows. The actual implementation is provided by the
//! git_adapter module which delegates to kodgen_git.

use crate::error::Result;
use semver::Version;
use std::future::Future;

/// Trait defining all required Git operations for release management
pub trait GitOperations {
    /// Create a commit with all current changes
    fn create_release_commit(
        &self,
        version: &Version,
        message: Option<String>,
    ) -> impl Future<Output = Result<CommitInfo>>;

    /// Create a version tag
    fn create_version_tag(
        &self,
        version: &Version,
        message: Option<String>,
    ) -> impl Future<Output = Result<TagInfo>>;

    /// Push commits and tags to remote
    fn push_to_remote(
        &self,
        remote_name: Option<&str>,
        push_tags: bool,
    ) -> impl Future<Output = Result<PushInfo>>;

    /// Check if working directory is clean
    fn is_working_directory_clean(&self) -> impl Future<Output = Result<bool>>;

    /// Get current branch information
    fn get_current_branch(&self) -> impl Future<Output = Result<BranchInfo>>;

    /// Reset to previous commit (rollback)
    fn reset_to_commit(
        &self,
        commit_id: &str,
        reset_type: ResetType,
    ) -> impl Future<Output = Result<()>>;

    /// Delete a tag (local and optionally remote)
    fn delete_tag(&self, tag_name: &str, delete_remote: bool) -> impl Future<Output = Result<()>>;

    /// Get commit history
    fn get_recent_commits(&self, count: usize) -> impl Future<Output = Result<Vec<CommitInfo>>>;

    /// Check if tag exists
    fn tag_exists(&self, tag_name: &str) -> impl Future<Output = Result<bool>>;

    /// Get remote information
    fn get_remotes(&self) -> impl Future<Output = Result<Vec<RemoteInfo>>>;

    /// Validate repository state for release
    fn validate_release_readiness(&self) -> impl Future<Output = Result<ValidationResult>>;

    /// Create and checkout a release branch
    fn create_release_branch(&self, version: &Version) -> impl Future<Output = Result<BranchInfo>>;

    /// Push a specific branch with tags to remote
    fn push_branch_with_tags(
        &self,
        branch_name: &str,
        remote_name: Option<&str>,
    ) -> impl Future<Output = Result<PushInfo>>;

    /// Checkout to a specific branch
    fn checkout_branch(&self, branch_name: &str) -> impl Future<Output = Result<()>>;

    /// Commit all uncommitted changes (stages and commits everything)
    fn commit_all_changes(&self, message: &str) -> impl Future<Output = Result<CommitInfo>>;

    /// Merge another branch into the current branch
    fn merge_branch(&self, branch_name: &str) -> impl Future<Output = Result<()>>;
}

/// Information about a Git commit
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Commit hash (full SHA)
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
    /// Parent commit hashes
    pub parents: Vec<String>,
}

/// Information about a Git tag
#[derive(Debug, Clone)]
pub struct TagInfo {
    /// Tag name
    pub name: String,
    /// Tag message (if annotated)
    pub message: Option<String>,
    /// Target commit hash
    pub target_commit: String,
    /// Tag timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether this is an annotated tag
    pub is_annotated: bool,
}

/// Information about a push operation
#[derive(Debug, Clone)]
pub struct PushInfo {
    /// Remote name that was pushed to
    pub remote_name: String,
    /// Number of commits pushed
    pub commits_pushed: usize,
    /// Number of tags pushed
    pub tags_pushed: usize,
    /// Any warnings or notes from the push
    pub warnings: Vec<String>,
}

/// Information about a Git branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current branch
    pub is_current: bool,
    /// Current commit hash
    pub commit_hash: String,
    /// Tracking remote branch (if any)
    pub upstream: Option<String>,
    /// Number of commits ahead of upstream
    pub ahead_count: Option<usize>,
    /// Number of commits behind upstream
    pub behind_count: Option<usize>,
}

/// Information about a Git remote
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    /// Remote name
    pub name: String,
    /// Fetch URL
    pub fetch_url: String,
    /// Push URL (may be different from fetch)
    pub push_url: String,
    /// Whether this remote is reachable
    pub is_reachable: bool,
}

/// Type of Git reset operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetType {
    /// Soft reset (keep changes in index)
    Soft,
    /// Mixed reset (keep changes in working directory)
    Mixed,
    /// Hard reset (discard all changes)
    Hard,
}

/// Result of Git validation for release readiness
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the repository is ready for release
    pub is_ready: bool,
    /// Issues that prevent release
    pub blocking_issues: Vec<String>,
    /// Warnings that should be addressed
    pub warnings: Vec<String>,
    /// Repository status summary
    pub status_summary: String,
}
