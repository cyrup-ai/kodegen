//! Branch deletion operation.
//!
//! This module provides functionality to delete branches.

use gix::bstr::ByteSlice;

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

use super::types::REFS_HEADS_PREFIX;

/// Delete a local branch.
///
/// Deletes the specified branch reference. Prevents deletion of the currently
/// checked-out branch as a safety measure.
///
/// # Parameters
///
/// - `name` - Branch name without "refs/heads/" prefix
/// - `force` - Reserved for future merge status checks (currently unused)
///
/// # Returns
///
/// - `Ok(())` - Branch successfully deleted
/// - `Err(GitError::InvalidInput)` - Tried to delete current branch
/// - `Err(GitError::BranchNotFound)` - Branch doesn't exist
/// - `Err(GitError::Gix)` - Other git operation errors
///
/// # Safety
///
/// This function will NOT delete the branch HEAD currently points to.
///
/// # Example
///
/// ```rust,ignore
/// delete_branch(repo, "feature-branch".to_string(), false).await?;
/// ```
pub fn delete_branch(repo: RepoHandle, name: String, _force: bool) -> AsyncTask<GitResult<()>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || {
        let branch_ref = format!("{REFS_HEADS_PREFIX}{name}");

        // CRITICAL SAFETY CHECK: Prevent deleting current branch
        if let Ok(head) = repo.head()
            && let Some(head_name) = head.referent_name()
            && let Ok(current_branch) = head_name.as_bstr().to_str()
            && current_branch == branch_ref
        {
            return Err(GitError::InvalidInput(format!(
                "Cannot delete current branch '{name}'"
            )));
        }

        // Find the branch reference
        let branch = repo
            .find_reference(&branch_ref)
            .map_err(|_| GitError::BranchNotFound(name.clone()))?;

        // Delete the reference (creates reflog entry automatically)
        branch.delete().map_err(|e| GitError::Gix(e.into()))?;

        Ok(())
    })
}
