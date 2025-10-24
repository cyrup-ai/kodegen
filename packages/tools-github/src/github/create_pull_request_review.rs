//! GitHub Pull Request review creation operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::{
    Octocrab,
    models::pulls::{Review, ReviewAction, ReviewComment},
};
use std::sync::Arc;

/// Options for creating a pull request review.
#[derive(Debug, Clone)]
pub struct CreatePullRequestReviewOptions {
    /// The review action: APPROVE, `REQUEST_CHANGES`, or COMMENT.
    pub event: ReviewAction,
    /// Optional review body/comment text.
    pub body: Option<String>,
    /// Optional commit ID that the review should be associated with.
    pub commit_id: Option<String>,
    /// Optional inline review comments.
    pub comments: Option<Vec<ReviewComment>>,
}

impl CreatePullRequestReviewOptions {
    /// Create options with the specified review action.
    #[must_use]
    pub fn new(event: ReviewAction) -> Self {
        Self {
            event,
            body: None,
            commit_id: None,
            comments: None,
        }
    }
}

/// Create a PR review (APPROVE, `REQUEST_CHANGES`, COMMENT).
pub(crate) fn create_pull_request_review(
    inner: Arc<Octocrab>,
    owner: impl Into<String>,
    repo: impl Into<String>,
    pr_number: u64,
    options: CreatePullRequestReviewOptions,
) -> AsyncTask<Result<Review, GitHubError>> {
    let (owner, repo) = (owner.into(), repo.into());

    spawn_task(async move {
        let mut review_data = serde_json::json!({
            "event": options.event,
        });

        if let Some(b) = options.body {
            review_data["body"] = serde_json::json!(b);
        }
        if let Some(cid) = options.commit_id {
            review_data["commit_id"] = serde_json::json!(cid);
        }
        if let Some(cmnts) = options.comments {
            review_data["comments"] = serde_json::json!(cmnts);
        }

        inner
            .post(
                format!("/repos/{owner}/{repo}/pulls/{pr_number}/reviews"),
                Some(&review_data),
            )
            .await
            .map_err(GitHubError::from)
    })
}
