//! Branch renaming operation.
//!
//! This module provides functionality to rename branches.

use gix::bstr::ByteSlice;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::{FullName, Target};

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

use super::helpers::is_valid_branch_name;
use super::types::REFS_HEADS_PREFIX;

/// Rename a local branch.
///
/// Creates a new branch with the new name pointing to the same commit as the old branch,
/// then deletes the old branch. If renaming the currently checked-out branch, also updates
/// HEAD to point to the new branch name.
///
/// # Parameters
///
/// - `old_name` - Current branch name (without "refs/heads/" prefix)
/// - `new_name` - New branch name (without "refs/heads/" prefix)
/// - `force` - If true, overwrite existing branch with new name
///
/// # Returns
///
/// - `Ok(())` - Branch successfully renamed
/// - `Err(GitError::InvalidInput)` - Invalid new name or new name exists without force
/// - `Err(GitError::BranchNotFound)` - Old branch doesn't exist
/// - `Err(GitError::Gix)` - Other git operation errors
///
/// # Example
///
/// ```rust,ignore
/// // Rename feature-old to feature-new
/// rename_branch(repo, "feature-old".to_string(), "feature-new".to_string(), false).await?;
///
/// // Rename with force (overwrites existing)
/// rename_branch(repo, "old".to_string(), "new".to_string(), true).await?;
/// ```
pub fn rename_branch(
    repo: RepoHandle,
    old_name: String,
    new_name: String,
    force: bool,
) -> AsyncTask<GitResult<()>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || {
        // Validate new branch name
        if !is_valid_branch_name(&new_name) {
            return Err(GitError::InvalidInput(format!(
                "Invalid branch name: '{new_name}'"
            )));
        }

        let old_ref = format!("{REFS_HEADS_PREFIX}{old_name}");
        let new_ref = format!("{REFS_HEADS_PREFIX}{new_name}");

        // Get old branch and its target OID
        let old_branch = repo
            .find_reference(&old_ref)
            .map_err(|_| GitError::BranchNotFound(old_name.clone()))?;

        // IMPORTANT: into_fully_peeled_id() CONSUMES the reference!
        let target_oid = old_branch
            .into_fully_peeled_id()
            .map_err(|e| GitError::Gix(e.into()))?;

        // Check if renaming current branch
        let is_current_branch = if let Ok(head) = repo.head() {
            if let Some(head_name) = head.referent_name() {
                head_name.as_bstr().to_str().ok() == Some(&old_ref)
            } else {
                false
            }
        } else {
            false
        };

        // Check if new branch already exists
        if repo.find_reference(&new_ref).is_ok() && !force {
            return Err(GitError::InvalidInput(format!(
                "Branch '{new_name}' already exists. Use force=true to overwrite."
            )));
        }

        // Create new reference pointing to same commit
        repo.reference(
            new_ref.as_str(),
            target_oid,
            PreviousValue::Any,
            format!("branch: renamed {old_name} to {new_name}"),
        )
        .map_err(|e| GitError::Gix(e.into()))?;

        // Find old reference again (consumed by into_fully_peeled_id)
        let old_branch = repo
            .find_reference(&old_ref)
            .map_err(|e| GitError::Gix(e.into()))?;

        // Delete old reference
        old_branch.delete().map_err(|e| GitError::Gix(e.into()))?;

        // Update HEAD if renaming current branch
        if is_current_branch {
            let head_name: FullName = "HEAD".try_into().map_err(|e| GitError::Gix(Box::new(e)))?;
            let new_full: FullName = new_ref
                .as_str()
                .try_into()
                .map_err(|e| GitError::Gix(Box::new(e)))?;

            repo.edit_reference(RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: format!("branch: renamed {old_name} to {new_name}").into(),
                    },
                    expected: PreviousValue::Any,
                    new: Target::Symbolic(new_full),
                },
                name: head_name,
                deref: false,
            })
            .map_err(|e| GitError::Gix(e.into()))?;
        }

        Ok(())
    })
}
