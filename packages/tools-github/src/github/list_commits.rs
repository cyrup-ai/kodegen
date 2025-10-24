//! GitHub repository commits listing operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::repos::RepoCommit};
use std::sync::Arc;

/// Options for listing commits in a repository.
#[derive(Debug, Clone, Default)]
pub struct ListCommitsOptions {
    /// SHA or branch to start listing commits from.
    pub sha: Option<String>,
    /// Only commits containing this file path will be returned.
    pub path: Option<String>,
    /// GitHub login or email address to filter commits by author.
    pub author: Option<String>,
    /// Only show commits after this date (ISO 8601 format).
    pub since: Option<String>,
    /// Only show commits before this date (ISO 8601 format).
    pub until: Option<String>,
    /// Page number of results to return.
    pub page: Option<u32>,
    /// Number of results per page (max 100).
    pub per_page: Option<u8>,
}

/// List commits in a repository.
pub(crate) fn list_commits(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    options: ListCommitsOptions,
) -> AsyncTask<Result<Vec<RepoCommit>, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();

    spawn_task(async move {
        let repos_handler = inner.repos(&owner, &repo);
        let mut request = repos_handler.list_commits();

        if let Some(sha_val) = options.sha {
            request = request.sha(sha_val);
        }

        if let Some(path_val) = options.path {
            request = request.path(path_val);
        }

        if let Some(author_val) = options.author {
            request = request.author(author_val);
        }

        if let Some(since_val) = options.since {
            let dt = chrono::DateTime::parse_from_rfc3339(&since_val).map_err(|e| {
                GitHubError::InvalidInput(format!("Invalid since date '{since_val}': {e}"))
            })?;
            request = request.since(dt.with_timezone(&chrono::Utc));
        }

        if let Some(until_val) = options.until {
            let dt = chrono::DateTime::parse_from_rfc3339(&until_val).map_err(|e| {
                GitHubError::InvalidInput(format!("Invalid until date '{until_val}': {e}"))
            })?;
            request = request.until(dt.with_timezone(&chrono::Utc));
        }

        if let Some(p) = options.page {
            request = request.page(p);
        }

        if let Some(pp) = options.per_page {
            request = request.per_page(pp);
        }

        let commits = request.send().await.map_err(GitHubError::from)?.items;

        Ok(commits)
    })
}
