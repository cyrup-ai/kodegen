//! GitHub Issue comment creation operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::issues::Comment};
use std::sync::Arc;

/// Add a comment to an existing issue.
pub(crate) fn add_issue_comment(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    issue_number: u64,
    body: impl Into<String>,
) -> AsyncTask<Result<Comment, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();
    let body = body.into();
    spawn_task(async move {
        let comment = inner
            .issues(&owner, &repo)
            .create_comment(issue_number, body)
            .await
            .map_err(GitHubError::from)?;
        Ok(comment)
    })
}
