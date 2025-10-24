//! GitHub Pull Request merge operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::Octocrab;
use std::sync::Arc;

/// Options for merging a pull request.
#[derive(Debug, Clone, Default)]
pub struct MergePullRequestOptions {
    /// Custom commit title for the merge commit.
    pub commit_title: Option<String>,
    /// Custom commit message for the merge commit.
    pub commit_message: Option<String>,
    /// SHA that pull request head must match to allow merge.
    pub sha: Option<String>,
    /// Merge method to use: "merge", "squash", or "rebase".
    pub merge_method: Option<String>,
}

/// Merge a pull request.
pub(crate) fn merge_pull_request(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    pull_number: u64,
    options: MergePullRequestOptions,
) -> AsyncTask<Result<serde_json::Value, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();

    spawn_task(async move {
        // Build the request body
        let mut body = serde_json::json!({});

        if let Some(title) = options.commit_title {
            body["commit_title"] = serde_json::json!(title);
        }
        if let Some(message) = options.commit_message {
            body["commit_message"] = serde_json::json!(message);
        }
        if let Some(sha_val) = options.sha {
            body["sha"] = serde_json::json!(sha_val);
        }
        if let Some(method) = options.merge_method {
            body["merge_method"] = serde_json::json!(method);
        }

        let url = format!("/repos/{owner}/{repo}/pulls/{pull_number}/merge");

        let result: serde_json::Value = inner
            .put(url, Some(&body))
            .await
            .map_err(GitHubError::from)?;

        Ok(result)
    })
}
