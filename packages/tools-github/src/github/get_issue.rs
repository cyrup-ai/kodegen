//! GitHub Issue retrieval operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::issues::Issue};
use std::sync::Arc;

/// Get a single issue.
pub(crate) fn get_issue(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    issue_number: u64,
) -> AsyncTask<Result<Issue, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();
    spawn_task(async move {
        let issue = inner
            .issues(&owner, &repo)
            .get(issue_number)
            .await
            .map_err(GitHubError::from)?;
        Ok(issue)
    })
}
