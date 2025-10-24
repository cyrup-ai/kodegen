//! GitHub commit retrieval operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::repos::RepoCommit};
use std::sync::Arc;

/// Get a specific commit by SHA.
pub(crate) fn get_commit(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    sha: impl Into<String>,
    page: Option<u32>,
    per_page: Option<u8>,
) -> AsyncTask<Result<RepoCommit, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();
    let sha = sha.into();

    spawn_task(async move {
        // Note: octocrab's get_commit returns detailed commit info
        // Page/per_page parameters are for the files list in the commit
        let mut url = format!("/repos/{owner}/{repo}/commits/{sha}");

        // Add pagination parameters if provided
        let mut params = vec![];
        if let Some(p) = page {
            params.push(format!("page={p}"));
        }
        if let Some(pp) = per_page {
            params.push(format!("per_page={pp}"));
        }
        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }

        let commit: RepoCommit = inner
            .get(url, None::<&()>)
            .await
            .map_err(GitHubError::from)?;

        Ok(commit)
    })
}
