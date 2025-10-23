//! Branch creation operation.
//!
//! This module provides functionality to create new branches.

use gix::bstr::ByteSlice;
use gix::refs::transaction::PreviousValue;

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

use super::helpers::{checkout_branch, is_valid_branch_name, parse_remote_branch, setup_tracking};
use super::types::{BranchOpts, REFS_HEADS_PREFIX};

/// Execute branch operation with the given options.
pub fn branch(repo: RepoHandle, opts: BranchOpts) -> AsyncTask<GitResult<()>> {
    let mut repo = repo.clone_inner();
    AsyncTask::spawn(move || {
        let BranchOpts {
            name,
            start_point,
            force,
            checkout,
            track,
        } = opts;

        // Validate branch name
        if !is_valid_branch_name(&name) {
            return Err(GitError::InvalidInput(format!(
                "Invalid branch name: '{name}'"
            )));
        }

        // Resolve start point (default to HEAD)
        let start_point_ref = start_point.as_deref().unwrap_or("HEAD");

        let parsed = repo
            .rev_parse(start_point_ref.as_bytes().as_bstr())
            .map_err(|e| {
                GitError::InvalidInput(format!("Invalid start point '{start_point_ref}': {e}"))
            })?;

        let target_id = parsed.single().ok_or_else(|| {
            GitError::InvalidInput(format!("Ambiguous start point: {start_point_ref}"))
        })?;

        // Detach the object ID for use in reference operations
        let target_oid = target_id.detach();

        // Check if branch already exists
        let branch_ref = format!("{REFS_HEADS_PREFIX}{name}");
        let branch_exists = repo.find_reference(&branch_ref).is_ok();

        if branch_exists && !force {
            return Err(GitError::InvalidInput(format!(
                "A branch named '{name}' already exists. Use force=true to overwrite."
            )));
        }

        // Determine constraint for reference update
        let constraint = if branch_exists {
            PreviousValue::Any
        } else {
            PreviousValue::MustNotExist
        };

        // Create reflog message
        let reflog_message = if branch_exists {
            format!("branch: Reset to {start_point_ref}")
        } else {
            format!("branch: Created from {start_point_ref}")
        };

        // Create or update the branch reference
        repo.reference(branch_ref.as_str(), target_oid, constraint, reflog_message)
            .map_err(|e| GitError::Gix(e.into()))?;

        // Handle tracking configuration
        if track {
            match start_point.as_deref().and_then(parse_remote_branch) {
                Some((remote_name, remote_branch)) => {
                    setup_tracking(&mut repo, &name, remote_name, remote_branch)?;
                }
                None => {
                    return Err(GitError::InvalidInput(format!(
                        "Cannot set up tracking: '{start_point_ref}' is not a remote branch"
                    )));
                }
            }
        }

        // Handle checkout operation
        if checkout {
            checkout_branch(&repo, &branch_ref, target_oid, force)?;
        }

        Ok(())
    })
}
