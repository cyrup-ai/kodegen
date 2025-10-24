//! GitHub Issue update operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::IssueState, models::issues::Issue};
use std::sync::Arc;

/// Request parameters for updating an issue
#[derive(Debug, Clone)]
pub struct UpdateIssueRequest {
    /// Repository owner (user or organization)
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Issue number to update
    pub issue_number: u64,
    /// New title for the issue
    pub title: Option<String>,
    /// New body/description for the issue
    pub body: Option<String>,
    /// New state (open or closed)
    pub state: Option<IssueState>,
    /// New labels
    pub labels: Option<Vec<String>>,
    /// New assignees
    pub assignees: Option<Vec<String>>,
    /// New milestone number
    pub milestone: Option<u64>,
}

/// Update an existing issue.
pub(crate) fn update_issue(
    inner: Arc<Octocrab>,
    request: UpdateIssueRequest,
) -> AsyncTask<Result<Issue, GitHubError>> {
    spawn_task(async move {
        let handler = inner.issues(&request.owner, &request.repo);
        let mut req = handler.update(request.issue_number);

        if let Some(ref t) = request.title {
            req = req.title(t.as_str());
        }
        if let Some(ref b) = request.body {
            req = req.body(b.as_str());
        }
        if let Some(s) = request.state {
            req = req.state(s);
        }
        if let Some(ref lbs) = request.labels {
            req = req.labels(lbs.as_slice());
        }
        if let Some(ref asgs) = request.assignees {
            req = req.assignees(asgs.as_slice());
        }
        if let Some(ms) = request.milestone {
            req = req.milestone(ms);
        }

        req.send().await.map_err(GitHubError::from)
    })
}
