//! Worktree locking operations.
//!
//! This module provides functionality to lock and unlock worktrees.

use std::path::PathBuf;

use gix::bstr::ByteSlice;

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

use super::helpers::find_worktree_by_path;
use super::types::WorktreeLockOpts;

/// Lock a worktree to prevent deletion.
///
/// # Examples
///
/// ```ignore
/// let opts = WorktreeLockOpts::new("/path/to/worktree")
///     .reason("Worktree on external drive");
/// worktree_lock(repo, opts).await?;
/// ```
pub fn worktree_lock(repo: RepoHandle, opts: WorktreeLockOpts) -> AsyncTask<GitResult<()>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || worktree_lock_impl(repo, opts))
}

fn worktree_lock_impl(repo: gix::Repository, opts: WorktreeLockOpts) -> GitResult<()> {
    // Get all linked worktrees
    let worktrees = repo.worktrees().map_err(GitError::Io)?;

    // Find the worktree by path
    let proxy = find_worktree_by_path(&worktrees, &opts.path)
        .ok_or_else(|| GitError::WorktreeNotFound(opts.path.display().to_string()))?;

    // Check if already locked
    if proxy.is_locked() {
        return Err(GitError::WorktreeLocked(
            proxy.id().to_str().unwrap_or("<unknown>").to_string(),
        ));
    }

    // Write lock file with optional reason
    let lock_file = proxy.git_dir().join("locked");
    let content = opts.reason.unwrap_or_default();

    std::fs::write(&lock_file, content).map_err(|e| {
        GitError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to create lock file at {}: {}",
                lock_file.display(),
                e
            ),
        ))
    })?;

    Ok(())
}

/// Unlock a worktree.
///
/// # Examples
///
/// ```ignore
/// worktree_unlock(repo, PathBuf::from("/path/to/worktree")).await?;
/// ```
pub fn worktree_unlock(repo: RepoHandle, path: PathBuf) -> AsyncTask<GitResult<()>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || worktree_unlock_impl(repo, path))
}

fn worktree_unlock_impl(repo: gix::Repository, path: PathBuf) -> GitResult<()> {
    // Get all linked worktrees
    let worktrees = repo.worktrees().map_err(GitError::Io)?;

    // Find the worktree by path
    let proxy = find_worktree_by_path(&worktrees, &path)
        .ok_or_else(|| GitError::WorktreeNotFound(path.display().to_string()))?;

    // Check if locked
    if !proxy.is_locked() {
        return Err(GitError::InvalidInput(format!(
            "Worktree at '{}' is not locked",
            path.display()
        )));
    }

    // Remove lock file
    let lock_file = proxy.git_dir().join("locked");

    std::fs::remove_file(&lock_file).map_err(|e| {
        GitError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to remove lock file at {}: {}",
                lock_file.display(),
                e
            ),
        ))
    })?;

    Ok(())
}
