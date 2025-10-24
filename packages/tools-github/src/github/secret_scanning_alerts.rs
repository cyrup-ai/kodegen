//! GitHub secret scanning alerts operations.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::repos::secret_scanning_alert::SecretScanningAlert};
use std::sync::Arc;

/// Get a specific secret scanning alert.
pub(crate) fn get_secret_scanning_alert(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    alert_number: u32,
) -> AsyncTask<Result<SecretScanningAlert, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();

    spawn_task(async move {
        let result = inner
            .repos(&owner, &repo)
            .secrets_scanning()
            .get_alert(alert_number)
            .await
            .map_err(GitHubError::from)?;
        Ok(result)
    })
}

/// List secret scanning alerts for a repository.
pub(crate) fn list_secret_scanning_alerts(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    state: Option<String>,
    secret_type: Option<String>,
    resolution: Option<String>,
) -> AsyncTask<Result<Vec<SecretScanningAlert>, GitHubError>> {
    let owner = owner.into();
    let repo = repo.into();

    spawn_task(async move {
        let repos = inner.repos(&owner, &repo);
        let mut handler = repos.secrets_scanning();

        if let Some(s) = state {
            handler = handler.state(s);
        }
        if let Some(st) = secret_type {
            handler = handler.secret_type(st);
        }
        if let Some(r) = resolution {
            // Note: octocrab's resolution takes Vec<String>
            handler = handler.resolution(vec![r]);
        }

        let page = handler.get_alerts().await.map_err(GitHubError::from)?;

        Ok(page.items)
    })
}
