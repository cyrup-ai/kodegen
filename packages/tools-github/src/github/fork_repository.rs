//! GitHub Repository forking operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::Repository};
use std::sync::Arc;

/// Fork a repository.
pub(crate) fn fork_repository(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    organization: Option<String>,
) -> AsyncTask<Result<Repository, GitHubError>> {
    let (owner, repo) = (owner.into(), repo.into());
    spawn_task(async move {
        let repo_handler = inner.repos(&owner, &repo);
        let mut fork_builder = repo_handler.create_fork();

        if let Some(org) = organization {
            fork_builder = fork_builder.organization(org);
        }

        fork_builder.send().await.map_err(GitHubError::from)
    })
}
