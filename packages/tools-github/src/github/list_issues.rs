//! GitHub Issues listing operation.

use crate::github::error::GitHubError;
use crate::runtime::{AsyncStream, EmitterBuilder};
use octocrab::models::IssueState;
use octocrab::models::issues::Issue;
use octocrab::{Octocrab, Page, params};
use std::sync::Arc;

/// Request parameters for listing issues
#[derive(Debug, Clone)]
pub struct ListIssuesRequest {
    /// Repository owner (user or organization)
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Filter by issue state (open, closed, all)
    pub state: Option<IssueState>,
    /// Filter by labels
    pub labels: Option<Vec<String>>,
    /// Sort field (created, updated, comments)
    pub sort: Option<String>,
    /// Sort direction (asc, desc)
    pub direction: Option<String>,
    /// Only issues updated after this time (RFC3339 timestamp)
    pub since: Option<String>,
    /// Page number for pagination
    pub page: Option<u32>,
    /// Results per page (max 100)
    pub per_page: Option<u8>,
}

/// List issues with optional filters. Uses a stream because the result can be large.
pub(crate) fn list_issues(
    inner: Arc<Octocrab>,
    request: ListIssuesRequest,
) -> AsyncStream<Result<Issue, GitHubError>> {
    let builder = EmitterBuilder::new(Box::new(move || {
        let request = request.clone();
        Box::pin(async move {
            let mut issues = Vec::new();
            let issues_handler = inner.issues(&request.owner, &request.repo);
            let mut req = issues_handler.list();

            if let Some(state) = request.state {
                let param_state = match state {
                    IssueState::Open => params::State::Open,
                    IssueState::Closed => params::State::Closed,
                    _ => params::State::All,
                };
                req = req.state(param_state);
            }
            if let Some(labels) = &request.labels {
                req = req.labels(labels);
            }
            if let Some(sort) = &request.sort {
                let sort_param = match sort.as_str() {
                    "created" => params::issues::Sort::Created,
                    "updated" => params::issues::Sort::Updated,
                    "comments" => params::issues::Sort::Comments,
                    _ => params::issues::Sort::Created,
                };
                req = req.sort(sort_param);
            }
            if let Some(direction) = &request.direction {
                let dir_param = match direction.as_str() {
                    "asc" => params::Direction::Ascending,
                    "desc" => params::Direction::Descending,
                    _ => params::Direction::Descending,
                };
                req = req.direction(dir_param);
            }
            if let Some(since) = &request.since {
                // Parse the string to DateTime
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(since) {
                    req = req.since(dt.with_timezone(&chrono::Utc));
                }
            }
            if let Some(page) = request.page {
                req = req.page(page);
            }
            if let Some(per_page) = request.per_page {
                req = req.per_page(per_page);
            }

            let mut page_res: Page<Issue> = req.send().await.map_err(GitHubError::from)?;
            issues.extend(page_res.items);

            while let Some(next_page) = inner.get_page::<Issue>(&page_res.next).await? {
                page_res = next_page;
                issues.extend(page_res.items);
            }
            Ok(issues)
        })
    }));
    builder.emit(|v| v, |_| {})
}
