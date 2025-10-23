//! Worktree listing operation.
//!
//! This module provides functionality to list all worktrees in a repository.

use crate::runtime::AsyncTask;
use crate::{GitResult, RepoHandle};

use super::helpers::read_head_info;
use super::types::WorktreeInfo;

/// List all worktrees in the repository.
///
/// Returns comprehensive information about all worktrees including the main
/// worktree and any linked worktrees.
///
/// # Examples
///
/// ```ignore
/// let worktrees = list_worktrees(repo).await?;
/// for wt in worktrees {
///     println!("{}: {}", wt.path.display(),
///              wt.head_branch.as_deref().unwrap_or("<detached>"));
/// }
/// ```
pub fn list_worktrees(repo: RepoHandle) -> AsyncTask<GitResult<Vec<WorktreeInfo>>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || {
        let mut all_worktrees = Vec::new();

        // Check if bare repository
        let is_bare = repo.is_bare();

        // Get main worktree (if not bare)
        if !is_bare && let Some(main_worktree) = repo.worktree() {
            let git_dir = repo.git_dir().to_path_buf();
            let path = main_worktree.base().to_path_buf();

            // Read HEAD information for main worktree (best effort)
            let (head_commit, head_branch, is_detached) =
                if let Ok(info) = read_head_info(&git_dir, &repo) {
                    info
                } else {
                    // HEAD file missing/corrupted - provide partial info
                    eprintln!(
                        "Warning: Failed to read HEAD for main worktree at {}",
                        git_dir.display()
                    );
                    (None, None, false)
                };

            let main_info = WorktreeInfo {
                path,
                git_dir,
                is_main: true,
                is_bare: false,
                head_commit,
                head_branch,
                is_locked: false, // Main worktree cannot be locked
                lock_reason: None,
                is_detached,
            };

            all_worktrees.push(main_info);
        }

        // Get linked worktrees
        let linked_worktrees = repo.worktrees().map_err(crate::GitError::Io)?;

        for proxy in linked_worktrees {
            // Get worktree path
            let path = proxy.base().map_err(crate::GitError::Io)?;
            let git_dir = proxy.git_dir().to_path_buf();

            // Read HEAD information for this worktree (best effort)
            let (head_commit, head_branch, is_detached) =
                if let Ok(info) = read_head_info(&git_dir, &repo) {
                    info
                } else {
                    // HEAD file missing/corrupted - provide partial info
                    eprintln!(
                        "Warning: Failed to read HEAD for worktree at {}",
                        path.display()
                    );
                    (None, None, false)
                };

            // Get lock status
            let is_locked = proxy.is_locked();
            let lock_reason = if is_locked {
                proxy.lock_reason().and_then(|bstr| {
                    use gix::bstr::ByteSlice;
                    bstr.to_str().ok().map(std::string::ToString::to_string)
                })
            } else {
                None
            };

            let worktree_info = WorktreeInfo {
                path,
                git_dir,
                is_main: false,
                is_bare: false,
                head_commit,
                head_branch,
                is_locked,
                lock_reason,
                is_detached,
            };

            all_worktrees.push(worktree_info);
        }

        Ok(all_worktrees)
    })
}
