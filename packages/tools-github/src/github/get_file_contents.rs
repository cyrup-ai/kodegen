//! GitHub File contents retrieval operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::repos::Content};
use std::sync::Arc;

/// Retrieve file or directory contents.
pub(crate) fn get_file_contents(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    path: impl Into<String>,
    reference: Option<String>,
) -> AsyncTask<Result<Vec<Content>, GitHubError>> {
    let (owner, repo, path) = (owner.into(), repo.into(), path.into());
    spawn_task(async move {
        let handler = inner.repos(&owner, &repo);
        let mut req = handler.get_content().path(&path);

        if let Some(r) = reference {
            req = req.r#ref(r);
        }

        let content_items = req.send().await.map_err(GitHubError::from)?;
        Ok(content_items.items)
    })
}
