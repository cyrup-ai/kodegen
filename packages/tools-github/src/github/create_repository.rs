//! GitHub Repository creation operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::Repository};
use std::sync::Arc;

/// Create a repository (user scope).
pub(crate) fn create_repository(
    inner: Arc<Octocrab>,
    name: impl Into<String>,
    description: Option<String>,
    private: Option<bool>,
    auto_init: Option<bool>,
) -> AsyncTask<Result<Repository, GitHubError>> {
    let name = name.into();
    spawn_task(async move {
        let mut body = serde_json::json!({
            "name": name,
        });

        if let Some(desc) = description {
            body["description"] = serde_json::json!(desc);
        }
        if let Some(privy) = private {
            body["private"] = serde_json::json!(privy);
        }
        if let Some(ai) = auto_init {
            body["auto_init"] = serde_json::json!(ai);
        }

        inner
            .post("/user/repos", Some(&body))
            .await
            .map_err(GitHubError::from)
    })
}
