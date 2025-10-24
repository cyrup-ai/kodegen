//! Git operations and management for release workflows.
//!
//! This module provides comprehensive Git integration using the gix library,
//! offering atomic operations, rollback capabilities, and release coordination.

mod git_adapter;
mod manager;
mod operations;

pub use git_adapter::KodegenGitOperations;
pub use manager::{
    BackupPoint, GitConfig, GitManager, ReleaseResult, RepositoryStats, RollbackResult,
};
pub use operations::{
    BranchInfo, CommitInfo, GitOperations, PushInfo, RemoteInfo, ResetType, TagInfo,
    ValidationResult,
};

use crate::error::Result;

/// Create a Git manager for the current directory
pub async fn create_git_manager() -> Result<GitManager> {
    GitManager::new(".").await
}

/// Create a Git manager with custom configuration
pub async fn create_git_manager_with_config(config: GitConfig) -> Result<GitManager> {
    GitManager::with_config(".", config).await
}

/// Quick validation of Git repository for release
pub async fn quick_git_validation() -> Result<ValidationResult> {
    let repo = KodegenGitOperations::open(".").await?;
    repo.validate_release_readiness().await
}

/// Check if current directory is a clean Git repository
pub async fn is_git_clean() -> Result<bool> {
    let repo = KodegenGitOperations::open(".").await?;
    repo.is_working_directory_clean().await
}

/// Get current Git branch information
pub async fn current_git_branch() -> Result<BranchInfo> {
    let repo = KodegenGitOperations::open(".").await?;
    repo.get_current_branch().await
}

/// Check if a version tag exists
pub async fn version_tag_exists(version: &semver::Version) -> Result<bool> {
    let repo = KodegenGitOperations::open(".").await?;
    let tag_name = format!("v{}", version);
    repo.tag_exists(&tag_name).await
}

/// Get recent commit history
pub async fn get_recent_commits(count: usize) -> Result<Vec<CommitInfo>> {
    let repo = KodegenGitOperations::open(".").await?;
    repo.get_recent_commits(count).await
}
