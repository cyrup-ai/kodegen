//! Worktree creation operation.
//!
//! This module provides functionality to create new linked worktrees.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use gix::bstr::ByteSlice;

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

use super::helpers::{check_branch_not_in_use, cleanup_failed_worktree, extract_branch_name};
use super::types::WorktreeAddOpts;

/// Create a new linked worktree.
///
/// # Examples
///
/// ```ignore
/// let opts = WorktreeAddOpts::new("/path/to/worktree")
///     .committish("feature-branch");
/// let path = worktree_add(repo, opts).await?;
/// ```
pub fn worktree_add(repo: RepoHandle, opts: WorktreeAddOpts) -> AsyncTask<GitResult<PathBuf>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || worktree_add_impl(repo, opts))
}

fn worktree_add_impl(repo: gix::Repository, opts: WorktreeAddOpts) -> GitResult<PathBuf> {
    // Phase 1: Validation

    // 1. Check worktree path doesn't exist (unless force)
    if opts.path.exists() && !opts.force {
        return Err(GitError::WorktreeAlreadyExists(opts.path.clone()));
    }

    // 2. Resolve committish to commit ID
    let committish_ref = opts.committish.as_deref().unwrap_or("HEAD");
    let parsed = repo
        .rev_parse(committish_ref.as_bytes().as_bstr())
        .map_err(|e| {
            GitError::InvalidInput(format!(
                "Failed to resolve committish '{committish_ref}': {e}"
            ))
        })?;

    let commit_id = parsed.single().ok_or_else(|| {
        GitError::InvalidInput(format!(
            "Committish '{committish_ref}' does not resolve to a single commit"
        ))
    })?;

    // 3. Check if branch is already checked out
    let branch_name = extract_branch_name(committish_ref);
    if let Some(branch) = branch_name {
        check_branch_not_in_use(&repo, branch)?;
    }

    // 4. Generate worktree name
    let worktree_name = opts
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            GitError::InvalidWorktreeName(format!("Invalid worktree path: {}", opts.path.display()))
        })?;

    // Phase 2: Create worktree git directory structure

    let worktree_git_dir = repo.common_dir().join("worktrees").join(worktree_name);

    // Ensure parent worktrees directory exists (idempotent, safe to call multiple times)
    let worktrees_parent = repo.common_dir().join("worktrees");
    if let Err(e) = std::fs::create_dir_all(&worktrees_parent) {
        return Err(GitError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to create worktrees parent directory at {}: {}",
                worktrees_parent.display(),
                e
            ),
        )));
    }

    // Helper closure for cleanup on error
    let cleanup = || cleanup_failed_worktree(&opts.path, &worktree_git_dir);

    // If force mode, remove both paths to start clean
    if opts.force {
        // Remove worktree directory if it exists
        if opts.path.exists() {
            std::fs::remove_dir_all(&opts.path).map_err(|e| {
                GitError::Io(std::io::Error::new(
                    e.kind(),
                    format!(
                        "Failed to remove existing directory at {} (force mode): {}",
                        opts.path.display(),
                        e
                    ),
                ))
            })?;
        }

        // Also remove admin directory if it exists from previous failed attempt
        // This is the KEY FIX - force mode now cleans up admin directory too
        if worktree_git_dir.exists() {
            std::fs::remove_dir_all(&worktree_git_dir).map_err(|e| {
                GitError::Io(std::io::Error::new(
                    e.kind(),
                    format!(
                        "Failed to remove existing worktree admin directory at {} (force mode): {}",
                        worktree_git_dir.display(),
                        e
                    ),
                ))
            })?;
        }
    }

    // Create worktree directory first (needed for canonicalize)
    if let Err(e) = std::fs::create_dir_all(&opts.path) {
        return Err(GitError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to create worktree directory at {}: {}",
                opts.path.display(),
                e
            ),
        )));
    }

    // Atomically create worktree git directory (prevents TOCTOU race condition)
    // Using create_dir (not create_dir_all) ensures atomic check-and-create
    match std::fs::create_dir(&worktree_git_dir) {
        Ok(()) => {} // Successfully created
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            cleanup();
            return Err(GitError::InvalidInput(format!(
                "Worktree with name '{}' already exists at {}. Use a different path name.",
                worktree_name,
                worktree_git_dir.display()
            )));
        }
        Err(e) => {
            cleanup();
            return Err(GitError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to create worktree git directory at {}: {}",
                    worktree_git_dir.display(),
                    e
                ),
            )));
        }
    }

    // Write gitdir file (absolute path to <worktree>/.git)
    let gitdir_file = worktree_git_dir.join("gitdir");
    let worktree_dotgit = match opts.path.canonicalize() {
        Ok(canonical) => canonical.join(".git"),
        Err(e) => {
            cleanup();
            return Err(GitError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to canonicalize worktree path {}: {}",
                    opts.path.display(),
                    e
                ),
            )));
        }
    };

    if let Err(e) = std::fs::write(&gitdir_file, format!("{}\n", worktree_dotgit.display())) {
        cleanup();
        return Err(GitError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to write gitdir file at {}: {}",
                gitdir_file.display(),
                e
            ),
        )));
    }

    // Write commondir file (relative path to main .git)
    if let Err(e) = std::fs::write(worktree_git_dir.join("commondir"), "../..\n") {
        cleanup();
        return Err(GitError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to write commondir file: {e}"),
        )));
    }

    // Write HEAD file
    let head_content = if opts.detach {
        format!("{commit_id}\n")
    } else if let Some(branch) = branch_name {
        format!("ref: refs/heads/{branch}\n")
    } else {
        format!("{commit_id}\n")
    };

    if let Err(e) = std::fs::write(worktree_git_dir.join("HEAD"), &head_content) {
        cleanup();
        return Err(GitError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to write HEAD file: {e}"),
        )));
    }

    // Phase 3: Write .git file (gitdir pointer)
    let dotgit_content = format!("gitdir: {}\n", worktree_git_dir.display());
    if let Err(e) = std::fs::write(opts.path.join(".git"), &dotgit_content) {
        cleanup();
        return Err(GitError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to write .git file: {e}"),
        )));
    }

    // Phase 4: Open worktree repository

    let worktree_repo = match gix::open(&opts.path) {
        Ok(r) => r,
        Err(e) => {
            cleanup();
            return Err(GitError::Gix(Box::new(e)));
        }
    };

    // Phase 5: Checkout files

    // Get commit and tree
    let commit = match worktree_repo.find_object(commit_id) {
        Ok(obj) => match obj.try_into_commit() {
            Ok(c) => c,
            Err(e) => {
                cleanup();
                return Err(GitError::Gix(Box::new(e)));
            }
        },
        Err(e) => {
            cleanup();
            return Err(GitError::Gix(Box::new(e)));
        }
    };

    let tree_id = match commit.tree_id() {
        Ok(id) => id,
        Err(e) => {
            cleanup();
            return Err(GitError::Gix(Box::new(e)));
        }
    };

    // Create index from tree
    let mut index = match worktree_repo.index_from_tree(&tree_id) {
        Ok(idx) => idx,
        Err(e) => {
            cleanup();
            return Err(GitError::Gix(
                format!("Failed to create index from tree: {e}").into(),
            ));
        }
    };

    // Configure checkout options
    let mut checkout_opts = match worktree_repo
        .checkout_options(gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping)
    {
        Ok(opts) => opts,
        Err(e) => {
            cleanup();
            return Err(GitError::Gix(
                format!("Failed to create checkout options: {e}").into(),
            ));
        }
    };

    checkout_opts.overwrite_existing = opts.force;
    checkout_opts.destination_is_initially_empty = true;

    // Perform checkout
    let objects = match worktree_repo.objects.clone().into_arc() {
        Ok(odb) => odb,
        Err(e) => {
            cleanup();
            return Err(GitError::Gix(
                format!("Failed to access object database: {e}").into(),
            ));
        }
    };

    let outcome = match gix::worktree::state::checkout(
        &mut index,
        opts.path.clone(),
        objects,
        &gix::progress::Discard,
        &gix::progress::Discard,
        &AtomicBool::new(false),
        checkout_opts,
    ) {
        Ok(o) => o,
        Err(e) => {
            cleanup();
            return Err(GitError::Gix(
                format!("Checkout operation failed: {e}").into(),
            ));
        }
    };

    // Handle errors
    if !outcome.errors.is_empty() {
        cleanup();
        return Err(GitError::Gix(
            format!(
                "Checkout failed with {} error(s): {}",
                outcome.errors.len(),
                outcome.errors[0].error
            )
            .into(),
        ));
    }

    // Write index to disk
    // Write index with proper locking and checksum
    if let Err(e) = index.write(gix::index::write::Options::default()) {
        cleanup();
        return Err(GitError::Gix(format!("Failed to write index: {e}").into()));
    }

    // Phase 6: Return result
    Ok(opts.path)
}
