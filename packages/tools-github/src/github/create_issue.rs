//! GitHub Issue creation operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::issues::Issue};
use std::sync::Arc;

/// Create a new issue.
pub(crate) fn create_issue(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    title: impl Into<String>,
    body: Option<String>,
    assignees: Option<Vec<String>>,
    labels: Option<Vec<String>>,
) -> AsyncTask<Result<Issue, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();
    let title = title.into();
    spawn_task(async move {
        let issues_handler = inner.issues(&owner, &repo);
        let mut req = issues_handler.create(title);

        req = req.body(body.unwrap_or_default());

        if let Some(asgs) = assignees {
            req = req.assignees(asgs);
        }
        if let Some(lbs) = labels {
            req = req.labels(lbs);
        }

        req.send().await.map_err(GitHubError::from)
    })
}
