//! Helper functions for worktree operations.
//!
//! This module contains internal utility functions used across worktree operations.

use std::path::Path;

use gix::hash::ObjectId;

use crate::{GitError, GitResult};

/// Read HEAD information from a git directory.
///
/// Returns (`head_commit`, `head_branch`, `is_detached`).
pub(super) fn read_head_info(
    git_dir: &Path,
    repo: &gix::Repository,
) -> GitResult<(Option<ObjectId>, Option<String>, bool)> {
    let head_file = git_dir.join("HEAD");

    // Read HEAD file
    let head_content = std::fs::read_to_string(&head_file).map_err(|e| {
        GitError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read HEAD file at {}: {}", head_file.display(), e),
        ))
    })?;

    let head_content = head_content.trim();

    // Parse HEAD content
    if let Some(ref_path) = head_content.strip_prefix("ref: ") {
        // Symbolic ref (attached HEAD)
        let branch_name = ref_path
            .strip_prefix("refs/heads/")
            .map(std::string::ToString::to_string);

        // Resolve the reference to get commit ID
        let commit_id = match repo.find_reference(ref_path) {
            Ok(reference) => {
                // Try to peel to commit
                match reference.into_fully_peeled_id() {
                    Ok(id) => Some(id.detach()),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        };

        Ok((commit_id, branch_name, false))
    } else {
        // Direct commit ID (detached HEAD)
        // Parse the object ID
        let commit_id = gix::hash::ObjectId::from_hex(head_content.as_bytes()).ok();

        Ok((commit_id, None, true))
    }
}

/// Extract branch name from committish string.
///
/// Returns Some(branch) for local branches, None for other refs or commits.
pub(super) fn extract_branch_name(committish: &str) -> Option<&str> {
    // Handle "refs/heads/branch" format
    if let Some(branch) = committish.strip_prefix("refs/heads/") {
        return Some(branch);
    }

    // Skip remote branches
    if committish.starts_with("origin/") || committish.starts_with("refs/remotes/") {
        return None;
    }

    // Skip tags
    if committish.starts_with("refs/tags/") {
        return None;
    }

    // Check if it looks like a commit SHA (40 hex chars or starts with hex)
    if committish.len() >= 7 && committish.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    // Otherwise assume it's a local branch name
    Some(committish)
}

/// Check if a branch is already checked out in any worktree.
pub(super) fn check_branch_not_in_use(repo: &gix::Repository, branch: &str) -> GitResult<()> {
    let branch_ref = format!("refs/heads/{branch}");

    // Check linked worktrees
    let worktrees = repo.worktrees().map_err(GitError::Io)?;
    for proxy in worktrees {
        let head_path = proxy.git_dir().join("HEAD");
        if let Ok(head_content) = std::fs::read_to_string(&head_path) {
            // Check for exact match to avoid false positives (e.g., "main-dev" matching "main")
            let head_trimmed = head_content.trim();
            let symbolic_ref = format!("ref: {branch_ref}");
            if head_trimmed == symbolic_ref {
                return Err(GitError::BranchInUse(branch.to_string()));
            }
        }
    }

    // Check main worktree
    let main_head = repo.git_dir().join("HEAD");
    if let Ok(head_content) = std::fs::read_to_string(&main_head) {
        // Check for exact match to avoid false positives (e.g., "main-dev" matching "main")
        let head_trimmed = head_content.trim();
        let symbolic_ref = format!("ref: {branch_ref}");
        if head_trimmed == symbolic_ref {
            return Err(GitError::BranchInUse(branch.to_string()));
        }
    }

    Ok(())
}

/// Clean up partially created worktree on failure (best effort).
pub(super) fn cleanup_failed_worktree(worktree_path: &Path, worktree_git_dir: &Path) {
    // Best effort cleanup - ignore errors
    let _ = std::fs::remove_dir_all(worktree_path);
    let _ = std::fs::remove_dir_all(worktree_git_dir);
}

/// Find a linked worktree by its checkout path.
///
/// Returns None if not found or if the path is the main worktree.
/// Only searches linked worktrees, not the main worktree.
pub(super) fn find_worktree_by_path<'a>(
    worktrees: &'a [gix::worktree::Proxy<'_>],
    search_path: &Path,
) -> Option<&'a gix::worktree::Proxy<'a>> {
    // Canonicalize search path for accurate comparison
    let canonical_search = search_path.canonicalize().ok()?;

    // Search through linked worktrees only
    worktrees.iter().find(|proxy| {
        if let Ok(base) = proxy.base()
            && let Ok(base_canonical) = base.canonicalize()
        {
            return base_canonical == canonical_search;
        }
        false
    })
}
