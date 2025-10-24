//! GitHub authenticated user retrieval operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::Author};
use std::sync::Arc;

/// Get details of the authenticated GitHub user.
///
/// Calls the `/user` endpoint to retrieve information about the user
/// associated with the provided authentication token.
///
/// # Example
/// ```rust
/// let client = GitHubClient::with_token("token")?;
/// let task = client.get_me();
/// let user = task.await??;
/// println!("Authenticated as: {}", user.login);
/// ```
pub(crate) fn get_me(inner: Arc<Octocrab>) -> AsyncTask<Result<Author, GitHubError>> {
    spawn_task(async move {
        let user = inner.current().user().await.map_err(GitHubError::from)?;
        Ok(user)
    })
}
