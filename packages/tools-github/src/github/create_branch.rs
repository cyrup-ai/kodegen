//! GitHub Branch creation operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::repos::Ref, params::repos::Reference};
use std::sync::Arc;

/// Create a new branch from an existing SHA.
pub(crate) fn create_branch(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    branch: impl Into<String>,
    sha: impl Into<String>,
) -> AsyncTask<Result<Ref, GitHubError>> {
    let (owner, repo, branch, sha) = (owner.into(), repo.into(), branch.into(), sha.into());
    spawn_task(async move {
        let reference = Reference::Branch(branch);
        inner
            .repos(&owner, &repo)
            .create_ref(&reference, sha)
            .await
            .map_err(GitHubError::from)
    })
}
