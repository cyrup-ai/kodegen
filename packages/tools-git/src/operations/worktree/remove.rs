//! Worktree removal operation.
//!
//! This module provides functionality to remove worktrees.

use gix::bstr::ByteSlice;

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

use super::helpers::find_worktree_by_path;
use super::types::WorktreeRemoveOpts;

/// Remove a worktree.
///
/// # Examples
///
/// ```ignore
/// let opts = WorktreeRemoveOpts::new("/path/to/worktree")
///     .force(true);
/// worktree_remove(repo, opts).await?;
/// ```
pub fn worktree_remove(repo: RepoHandle, opts: WorktreeRemoveOpts) -> AsyncTask<GitResult<()>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || worktree_remove_impl(repo, opts))
}

fn worktree_remove_impl(repo: gix::Repository, opts: WorktreeRemoveOpts) -> GitResult<()> {
    // Step 1: Find worktree by path
    let worktrees = repo.worktrees().map_err(GitError::Io)?;

    let proxy = find_worktree_by_path(&worktrees, &opts.path)
        .ok_or_else(|| GitError::WorktreeNotFound(opts.path.display().to_string()))?;

    // Step 2: Validate not main worktree
    // Main worktree has git_dir directly in .git, not in .git/worktrees/<name>
    let is_main = proxy
        .git_dir()
        .parent()
        .and_then(|p| p.file_name())
        .is_none_or(|n| n != "worktrees");

    if is_main {
        return Err(GitError::CannotModifyMainWorktree);
    }

    // Step 3: Check lock status
    if proxy.is_locked() && !opts.force {
        return Err(GitError::WorktreeLocked(
            proxy.id().to_str().unwrap_or("<unknown>").to_string(),
        ));
    }

    // Step 4: Get paths before consuming proxy
    let git_dir = proxy.git_dir().to_path_buf();
    let worktree_path = proxy.base().map_err(GitError::Io)?;

    // Step 5: Remove worktree directory (physical files)
    if worktree_path.exists() {
        std::fs::remove_dir_all(&worktree_path).map_err(|e| {
            GitError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to remove worktree directory at {}: {}",
                    worktree_path.display(),
                    e
                ),
            ))
        })?;
    }

    // Step 6: Remove admin directory (.git/worktrees/<name>)
    std::fs::remove_dir_all(&git_dir).map_err(|e| {
        GitError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to remove worktree git directory at {}: {}",
                git_dir.display(),
                e
            ),
        ))
    })?;

    Ok(())
}
