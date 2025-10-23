//! Branch listing operation.
//!
//! This module provides functionality to list branches.

use gix::bstr::ByteSlice;

use crate::runtime::AsyncTask;
use crate::{GitError, GitResult, RepoHandle};

use super::types::REFS_HEADS_PREFIX;

/// List all local branches in the repository.
///
/// Returns a vector of branch names without the "refs/heads/" prefix.
/// Only local branches are included (refs/heads/*), not remote-tracking branches.
///
/// # Returns
///
/// - `Ok(Vec<String>)` - List of local branch names
/// - `Err(GitError)` - If reference iteration fails
///
/// # Example
///
/// ```rust,ignore
/// let branches = list_branches(repo).await?;
/// for branch in branches {
///     println!("Branch: {}", branch);
/// }
/// ```
pub fn list_branches(repo: RepoHandle) -> AsyncTask<GitResult<Vec<String>>> {
    let repo = repo.clone_inner();
    AsyncTask::spawn(move || {
        let mut branches = Vec::new();

        // Get reference platform and local branches iterator
        let refs = repo.references().map_err(|e| GitError::Gix(e.into()))?;

        let iter = refs.local_branches().map_err(|e| GitError::Gix(e.into()))?;

        // Iterate over all local branches
        for reference_result in iter {
            let reference = reference_result.map_err(GitError::Gix)?;

            // Get reference name as BStr
            let name_bytes = reference.name().as_bstr();

            // Convert to str and strip prefix
            if let Ok(name) = name_bytes.to_str()
                && let Some(branch_name) = name.strip_prefix(REFS_HEADS_PREFIX)
            {
                branches.push(branch_name.to_string());
            }
            // Silently skip non-UTF-8 branch names
        }

        Ok(branches)
    })
}
