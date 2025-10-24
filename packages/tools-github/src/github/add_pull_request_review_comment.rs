//! GitHub Pull Request review comment creation operation.

use crate::github::{error::GitHubError, util::spawn_task};
use crate::runtime::AsyncTask;
use octocrab::Octocrab;
use std::sync::Arc;

/// Request parameters for adding a pull request review comment
#[derive(Debug, Clone)]
pub struct AddPullRequestReviewCommentRequest {
    /// Repository owner (user or organization)
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Pull request number
    pub pr_number: u64,
    /// Comment body text
    pub body: String,
    /// Commit SHA to comment on
    pub commit_id: Option<String>,
    /// File path to comment on
    pub path: Option<String>,
    /// Line number in the diff to comment on
    pub line: Option<u32>,
    /// Side of the diff (LEFT or RIGHT)
    pub side: Option<String>,
    /// Start line for multi-line comments
    pub start_line: Option<u32>,
    /// Side of the start line
    pub start_side: Option<String>,
    /// Subject type (line or file)
    pub subject_type: Option<String>,
    /// Comment ID to reply to (for threaded comments)
    pub in_reply_to: Option<u64>,
}

/// Add a single review comment (or reply).
pub(crate) fn add_pull_request_review_comment(
    inner: Arc<Octocrab>,
    request: AddPullRequestReviewCommentRequest,
) -> AsyncTask<Result<octocrab::models::pulls::ReviewComment, GitHubError>> {
    spawn_task(async move {
        // If this is a reply to an existing comment, use reply_to_comment
        if let Some(comment_id) = request.in_reply_to {
            return inner
                .pulls(&request.owner, &request.repo)
                .reply_to_comment(
                    request.pr_number,
                    octocrab::models::CommentId(comment_id),
                    request.body,
                )
                .await
                .map_err(GitHubError::from);
        }

        // Otherwise, create a new review comment via direct POST
        let mut comment_data = serde_json::json!({
            "body": request.body,
        });

        if let Some(cid) = request.commit_id {
            comment_data["commit_id"] = serde_json::json!(cid);
        }
        if let Some(p) = request.path {
            comment_data["path"] = serde_json::json!(p);
        }
        if let Some(l) = request.line {
            comment_data["line"] = serde_json::json!(l);
        }
        if let Some(s) = request.side {
            comment_data["side"] = serde_json::json!(s);
        }
        if let Some(sl) = request.start_line {
            comment_data["start_line"] = serde_json::json!(sl);
        }
        if let Some(ss) = request.start_side {
            comment_data["start_side"] = serde_json::json!(ss);
        }
        if let Some(st) = request.subject_type {
            comment_data["subject_type"] = serde_json::json!(st);
        }

        let owner = &request.owner;
        let repo = &request.repo;
        let pr_number = request.pr_number;

        inner
            .post(
                format!("/repos/{owner}/{repo}/pulls/{pr_number}/comments"),
                Some(&comment_data),
            )
            .await
            .map_err(GitHubError::from)
    })
}
