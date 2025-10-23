//! Worktree pruning operation.
//!
//! This module provides functionality to prune stale worktree administrative files.

use gix::bstr::ByteSlice;

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

/// Prune stale worktree administrative files.
///
/// Returns a list of pruned worktree names.
///
/// # Examples
///
/// ```ignore
/// let pruned = worktree_prune(repo).await?;
/// println!("Pruned {} stale worktrees", pruned.len());
/// ```
pub fn worktree_prune(repo: RepoHandle) -> AsyncTask<GitResult<Vec<String>>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || worktree_prune_impl(repo))
}

fn worktree_prune_impl(repo: gix::Repository) -> GitResult<Vec<String>> {
    let mut pruned = Vec::new();

    // Step 1: Get all linked worktrees
    let worktrees = repo.worktrees().map_err(GitError::Io)?;

    // Step 2: Check each worktree for validity
    for proxy in worktrees {
        let should_prune = match proxy.base() {
            Ok(base) => {
                // Prune if:
                // - Worktree directory doesn't exist
                // - .git file is missing or invalid
                !base.exists() || !base.join(".git").exists()
            }
            Err(_) => {
                // Prune if gitdir file is invalid/missing
                true
            }
        };

        if should_prune {
            let git_dir = proxy.git_dir().to_path_buf();
            let name = proxy.id().to_str().unwrap_or("<unknown>").to_string();

            // Remove stale worktree admin directory (best effort)
            match std::fs::remove_dir_all(&git_dir) {
                Ok(()) => {
                    pruned.push(name);
                }
                Err(e) => {
                    // Log warning but continue pruning others
                    eprintln!(
                        "Warning: Failed to prune worktree '{}' at {}: {}",
                        name,
                        git_dir.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(pruned)
}
