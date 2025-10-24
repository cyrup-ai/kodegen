//! GitHub Copilot review request operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::Octocrab;
use std::sync::Arc;

/// Request Copilot review (experimental).
pub(crate) fn request_copilot_review(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    pr_number: u64,
) -> AsyncTask<Result<(), GitHubError>> {
    let (owner, repo) = (owner.into(), repo.into());

    spawn_task(async move {
        // Raw endpoint until Octocrab exposes it natively.
        let route = format!("repos/{owner}/{repo}/pulls/{pr_number}/copilot-review");

        inner
            .post::<(), ()>(route, Some(&()))
            .await
            .map_err(GitHubError::from)?;
        Ok(())
    })
}
