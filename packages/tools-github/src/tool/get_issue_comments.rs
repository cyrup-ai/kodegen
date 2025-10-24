//! GitHub issue comments retrieval tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{GetIssueCommentsArgs, GetIssueCommentsPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

/// Tool for fetching all comments on a GitHub issue
#[derive(Clone)]
pub struct GetIssueCommentsTool;

impl Tool for GetIssueCommentsTool {
    type Args = GetIssueCommentsArgs;
    type PromptArgs = GetIssueCommentsPromptArgs;

    fn name() -> &'static str {
        "get_issue_comments"
    }

    fn description() -> &'static str {
        "Fetch all comments for a GitHub issue. Returns an array of comment objects \
         including author, body, timestamps, and metadata. Comments are returned in \
         chronological order. Requires GITHUB_TOKEN environment variable."
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true
    }

    fn open_world() -> bool {
        true // Calls external GitHub API
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Call API wrapper (returns AsyncStream)
        let mut comment_stream =
            client.get_issue_comments(args.owner, args.repo, args.issue_number);

        // Collect stream results
        let mut comments = Vec::new();
        while let Some(result) = comment_stream.next().await {
            let comment =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            comments.push(comment);
        }

        // Return serialized comments
        Ok(json!({ "comments": comments, "count": comments.len() }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I fetch all comments for a GitHub issue?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the get_issue_comments tool to fetch all comments for an issue:\n\n\
                     Basic usage:\n\
                     get_issue_comments({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42})\n\n\
                     The returned comments array includes:\n\
                     - id: Comment ID\n\
                     - body: Comment text (Markdown)\n\
                     - user: Author information (login, avatar_url, etc.)\n\
                     - created_at: When comment was created\n\
                     - updated_at: When comment was last edited\n\
                     - html_url: Link to comment on GitHub\n\
                     - author_association: Relationship to repo (OWNER, CONTRIBUTOR, etc.)\n\n\
                     Comment ordering:\n\
                     - Comments are returned in chronological order (oldest first)\n\
                     - Use the created_at timestamp to determine comment age\n\
                     - The first comment is always the oldest\n\n\
                     Working with comments:\n\
                     - Identify authors by user.login field\n\
                     - Check author_association to see if author is repo owner/maintainer\n\
                     - Body contains Markdown - may include code blocks, @mentions, etc.\n\
                     - Updated_at differs from created_at if comment was edited\n\n\
                     Use cases:\n\
                     - Read discussion history on an issue\n\
                     - Find specific feedback from team members\n\
                     - Check if issue has been commented on recently\n\
                     - Extract action items from comments\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos\n\
                     - Works for both issues and pull requests",
                ),
            },
        ])
    }
}
