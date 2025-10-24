//! GitHub code scanning alerts operations.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::Octocrab;
use std::sync::Arc;

/// Get a specific code scanning alert.
pub(crate) fn get_code_scanning_alert(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    alert_number: u64,
) -> AsyncTask<Result<serde_json::Value, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();

    spawn_task(async move {
        let url = format!("/repos/{owner}/{repo}/code-scanning/alerts/{alert_number}");
        let result: serde_json::Value = inner
            .get(url, None::<&()>)
            .await
            .map_err(GitHubError::from)?;
        Ok(result)
    })
}

/// List code scanning alerts for a repository.
pub(crate) fn list_code_scanning_alerts(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    state: Option<String>,
    ref_name: Option<String>,
    tool_name: Option<String>,
    severity: Option<String>,
) -> AsyncTask<Result<Vec<serde_json::Value>, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();

    spawn_task(async move {
        let mut url = format!("/repos/{owner}/{repo}/code-scanning/alerts");
        let mut params = vec![];

        if let Some(s) = state {
            params.push(format!("state={s}"));
        }
        if let Some(r) = ref_name {
            params.push(format!("ref={r}"));
        }
        if let Some(t) = tool_name {
            params.push(format!("tool_name={t}"));
        }
        if let Some(sev) = severity {
            params.push(format!("severity={sev}"));
        }

        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }

        let results: Vec<serde_json::Value> = inner
            .get(url, None::<&()>)
            .await
            .map_err(GitHubError::from)?;
        Ok(results)
    })
}
