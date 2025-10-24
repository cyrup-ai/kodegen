//! GitHub Pull Request creation operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::pulls::PullRequest};
use std::sync::Arc;

/// Request parameters for creating a pull request
#[derive(Debug, Clone)]
pub struct CreatePullRequestRequest {
    /// Repository owner (user or organization)
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Pull request title
    pub title: String,
    /// Pull request body/description
    pub body: Option<String>,
    /// Branch or commit SHA where changes are implemented
    pub head: String,
    /// Branch to merge into
    pub base: String,
    /// Whether to create as draft pull request
    pub draft: Option<bool>,
    /// Whether maintainers can modify the pull request
    pub maintainer_can_modify: Option<bool>,
}

/// Create a pull-request.
pub(crate) fn create_pull_request(
    inner: Arc<Octocrab>,
    request: CreatePullRequestRequest,
) -> AsyncTask<Result<PullRequest, GitHubError>> {
    spawn_task(async move {
        let handler = inner.pulls(&request.owner, &request.repo);
        let mut req = handler.create(&request.head, &request.base, &request.title);

        req = req.body(request.body.unwrap_or_default());

        if let Some(d) = request.draft {
            req = req.draft(d);
        }
        if let Some(mcm) = request.maintainer_can_modify {
            req = req.maintainer_can_modify(mcm);
        }

        req.send().await.map_err(GitHubError::from)
    })
}
