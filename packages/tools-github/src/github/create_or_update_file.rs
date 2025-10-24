//! GitHub File creation/update operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{Octocrab, models::repos::FileUpdate as FileUpdateResponse};
use std::sync::Arc;

/// Request parameters for creating or updating a file
#[derive(Debug, Clone)]
pub struct CreateOrUpdateFileRequest {
    /// Repository owner (user or organization)
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Path to the file in the repository
    pub path: String,
    /// Commit message
    pub message: String,
    /// File content (will be base64 encoded by octocrab)
    pub content: String,
    /// Branch to commit to (defaults to repository default branch)
    pub branch: Option<String>,
    /// SHA of the file being updated (required for updates, omit for creates)
    pub sha: Option<String>,
}

/// Create **or** update a single file.
pub(crate) fn create_or_update_file(
    inner: Arc<Octocrab>,
    request: CreateOrUpdateFileRequest,
) -> AsyncTask<Result<FileUpdateResponse, GitHubError>> {
    spawn_task(async move {
        let handler = inner.repos(&request.owner, &request.repo);
        let mut builder = if let Some(existing_sha) = request.sha {
            handler.update_file(
                &request.path,
                &request.message,
                request.content.as_bytes(),
                existing_sha,
            )
        } else {
            handler.create_file(&request.path, &request.message, request.content.as_bytes())
        };

        if let Some(b) = request.branch {
            builder = builder.branch(b);
        }

        builder.send().await.map_err(GitHubError::from)
    })
}
