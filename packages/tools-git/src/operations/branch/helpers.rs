//! Helper functions for branch operations.
//!
//! This module contains internal utility functions used across branch operations.

use std::borrow::Cow;
use std::sync::atomic::AtomicBool;

use gix::bstr::ByteSlice;

use crate::{GitError, GitResult};

use super::types::{REFS_HEADS_PREFIX, REFS_REMOTES_PREFIX};

/// Validate branch name according to git ref naming rules.
///
/// Returns true if the name is valid for a git branch.
/// Zero allocation validation using byte slice operations.
#[inline]
pub(super) fn is_valid_branch_name(name: &str) -> bool {
    let bytes = name.as_bytes();

    // Must not be empty
    if bytes.is_empty() {
        return false;
    }

    // Must not start with '.' or '/'
    if bytes[0] == b'.' || bytes[0] == b'/' {
        return false;
    }

    // Must not end with '/' or '.lock'
    if bytes[bytes.len() - 1] == b'/' {
        return false;
    }
    if name.ends_with(".lock") {
        return false;
    }

    // Check for forbidden patterns and characters
    let mut prev_byte = 0u8;
    for (i, &byte) in bytes.iter().enumerate() {
        // ASCII control characters (0x00-0x1F, 0x7F)
        if byte <= 0x1F || byte == 0x7F {
            return false;
        }

        // Space
        if byte == b' ' {
            return false;
        }

        // Forbidden characters: ~^:?*[\
        match byte {
            b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\' => return false,
            _ => {}
        }

        // Consecutive slashes
        if byte == b'/' && prev_byte == b'/' {
            return false;
        }

        // Pattern /.
        if byte == b'.' && prev_byte == b'/' {
            return false;
        }

        // Check for .. pattern
        if i > 0 && byte == b'.' && prev_byte == b'.' {
            return false;
        }

        // Check for @{ pattern
        if i > 0 && byte == b'{' && prev_byte == b'@' {
            return false;
        }

        prev_byte = byte;
    }

    // Reserved names
    !matches!(
        name,
        "HEAD" | "FETCH_HEAD" | "ORIG_HEAD" | "MERGE_HEAD" | "-"
    )
}

/// Parse remote branch specification into (`remote_name`, `branch_name`).
///
/// Handles formats:
/// - "refs/remotes/origin/main" -> Some(("origin", "main"))
/// - "origin/feature/sub" -> Some(("origin", "feature/sub"))
/// - "main" -> None (local branch)
///
/// Returns borrowed string slices to avoid allocation.
#[inline]
pub(super) fn parse_remote_branch(spec: &str) -> Option<(&str, &str)> {
    // Handle full remote ref: refs/remotes/<remote>/<branch>
    if let Some(stripped) = spec.strip_prefix(REFS_REMOTES_PREFIX) {
        return stripped
            .split_once('/')
            .filter(|(remote, branch)| !remote.is_empty() && !branch.is_empty());
    }

    // Handle short form: <remote>/<branch>
    // Exclude refs/* to avoid matching local refs
    if !spec.starts_with("refs/") {
        return spec
            .split_once('/')
            .filter(|(remote, branch)| !remote.is_empty() && !branch.is_empty());
    }

    None
}

/// Setup tracking configuration for a branch.
///
/// Writes branch.<name>.remote and branch.<name>.merge to repository config.
#[inline]
pub(super) fn setup_tracking(
    repo: &mut gix::Repository,
    branch_name: &str,
    remote_name: &str,
    remote_branch: &str,
) -> GitResult<()> {
    use gix::config::parse::section::ValueName;

    // Get mutable config snapshot
    let mut config = repo.config_snapshot_mut();

    // Create branch section with branch name as subsection
    let mut section = config
        .new_section("branch", Some(Cow::Owned(branch_name.to_string().into())))
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    // Set branch.<name>.remote = <remote_name>
    let remote_key = ValueName::try_from("remote").map_err(|e| GitError::Gix(Box::new(e)))?;
    section.push(remote_key, Some(remote_name.as_bytes().as_bstr()));

    // Set branch.<name>.merge = refs/heads/<remote_branch>
    let merge_key = ValueName::try_from("merge").map_err(|e| GitError::Gix(Box::new(e)))?;
    let merge_ref = format!("{REFS_HEADS_PREFIX}{remote_branch}");
    section.push(merge_key, Some(merge_ref.as_bytes().as_bstr()));

    // Commit configuration changes to persist to .git/config
    drop(section);
    config.commit().map_err(|e| GitError::Gix(Box::new(e)))?;

    Ok(())
}

/// Checkout working tree to match the given tree, then update HEAD symbolically.
///
/// This performs a complete checkout operation:
/// 1. Create index from target tree
/// 2. Checkout files to working directory
/// 3. Write updated index
/// 4. Update HEAD to point symbolically to the branch
#[inline]
pub(super) fn checkout_branch(
    repo: &gix::Repository,
    branch_ref: &str,
    target_id: gix::hash::ObjectId,
    force: bool,
) -> GitResult<()> {
    use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
    use gix::refs::{FullName, Target};

    // Validate not a bare repository
    let worktree = repo.worktree().ok_or_else(|| {
        GitError::InvalidInput("Cannot checkout branch in bare repository".into())
    })?;

    // Get target commit and its tree
    let commit = repo
        .find_object(target_id)
        .map_err(|e| GitError::Gix(e.into()))?
        .try_into_commit()
        .map_err(|_| GitError::InvalidInput("Target does not point to a commit".into()))?;

    let tree_id = commit.tree_id().map_err(|e| GitError::Gix(e.into()))?;

    // Create index from target tree
    let mut index = repo
        .index_from_tree(&tree_id)
        .map_err(|e| GitError::Gix(e.into()))?;

    // Configure checkout options
    let mut checkout_opts = repo
        .checkout_options(gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping)
        .map_err(|e| GitError::Gix(e.into()))?;
    checkout_opts.overwrite_existing = force;
    checkout_opts.destination_is_initially_empty = false;

    // Perform the actual file checkout
    let outcome = gix::worktree::state::checkout(
        &mut index,
        worktree.base(),
        repo.objects
            .clone()
            .into_arc()
            .map_err(|e| GitError::Gix(e.into()))?,
        &gix::progress::Discard,
        &gix::progress::Discard,
        &AtomicBool::new(false),
        checkout_opts,
    )
    .map_err(|e| GitError::Gix(e.into()))?;

    // Handle checkout errors
    if !outcome.errors.is_empty() {
        return Err(GitError::InvalidInput(format!(
            "Checkout encountered {} error(s)",
            outcome.errors.len()
        )));
    }

    // Handle collisions (unless forced)
    if !outcome.collisions.is_empty() && !force {
        return Err(GitError::InvalidInput(format!(
            "Checkout encountered {} collision(s). Use force=true to override.",
            outcome.collisions.len()
        )));
    }

    // Write updated index to disk with proper locking and checksum
    index
        .write(gix::index::write::Options::default())
        .map_err(|e| GitError::Gix(e.into()))?;

    // Update HEAD to point symbolically to the branch
    let head_name: FullName = "HEAD".try_into().map_err(|e| GitError::Gix(Box::new(e)))?;

    let branch_full_name: FullName = branch_ref
        .try_into()
        .map_err(|e| GitError::Gix(Box::new(e)))?;

    let branch_name = branch_ref
        .strip_prefix(REFS_HEADS_PREFIX)
        .unwrap_or(branch_ref);

    repo.edit_reference(RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: false,
                message: format!("checkout: moving from HEAD to {branch_name}").into(),
            },
            expected: PreviousValue::Any,
            new: Target::Symbolic(branch_full_name),
        },
        name: head_name,
        deref: false,
    })
    .map_err(|e| GitError::Gix(e.into()))?;

    Ok(())
}
