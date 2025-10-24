//! GitHub repository branches listing operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::repos::Branch};
use std::sync::Arc;

/// List branches in a repository.
pub(crate) fn list_branches(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    page: Option<u32>,
    per_page: Option<u8>,
) -> AsyncTask<Result<Vec<Branch>, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();

    spawn_task(async move {
        let repos_handler = inner.repos(&owner, &repo);
        let mut request = repos_handler.list_branches();

        if let Some(p) = page {
            request = request.page(p);
        }

        if let Some(pp) = per_page {
            request = request.per_page(pp);
        }

        let branches = request.send().await.map_err(GitHubError::from)?.items;

        Ok(branches)
    })
}
