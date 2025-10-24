//! GitHub Pull Request update operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::pulls::PullRequest, params};
use std::sync::Arc;

/// Options for updating a pull request.
#[derive(Debug, Clone, Default)]
pub struct UpdatePullRequestOptions {
    /// New title for the pull request.
    pub title: Option<String>,
    /// New body text for the pull request.
    pub body: Option<String>,
    /// New state (Open or Closed).
    pub state: Option<params::pulls::State>,
    /// New base branch.
    pub base: Option<String>,
    /// Whether maintainer can modify the PR.
    pub maintainer_can_modify: Option<bool>,
}

/// Update an existing pull-request.
pub(crate) fn update_pull_request(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    pr_number: u64,
    options: UpdatePullRequestOptions,
) -> AsyncTask<Result<PullRequest, GitHubError>> {
    let (owner, repo) = (owner.into(), repo.into());

    spawn_task(async move {
        let pulls_handler = inner.pulls(&owner, &repo);
        let mut req = pulls_handler.update(pr_number);

        if let Some(t) = options.title {
            req = req.title(t);
        }
        if let Some(b) = options.body {
            req = req.body(b);
        }
        if let Some(s) = options.state {
            req = req.state(s);
        }
        if let Some(bs) = options.base {
            req = req.base(bs);
        }
        if let Some(mcm) = options.maintainer_can_modify {
            req = req.maintainer_can_modify(mcm);
        }

        req.send().await.map_err(GitHubError::from)
    })
}
