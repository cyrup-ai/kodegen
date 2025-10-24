//! GitHub issue retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{GetIssueArgs, GetIssuePromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

/// Tool for fetching a GitHub issue by number
#[derive(Clone)]
pub struct GetIssueTool;

impl Tool for GetIssueTool {
    type Args = GetIssueArgs;
    type PromptArgs = GetIssuePromptArgs;

    fn name() -> &'static str {
        "get_issue"
    }

    fn description() -> &'static str {
        "Fetch a single GitHub issue by number. Returns detailed issue information including \
         title, body, state, labels, assignees, comments count, and timestamps. \
         Requires GITHUB_TOKEN environment variable."
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

        // Call API wrapper (returns AsyncTask<Result<Issue, GitHubError>>)
        // The .await returns Result<Result<Issue, GitHubError>, RecvError>
        let task_result = client
            .get_issue(args.owner, args.repo, args.issue_number)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let issue =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Return serialized issue
        Ok(serde_json::to_value(&issue)?)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I fetch a specific GitHub issue?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the get_issue tool to fetch a GitHub issue by its number:\n\n\
                     Basic usage:\n\
                     get_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42})\n\n\
                     The returned issue object includes:\n\
                     - number: Issue number\n\
                     - title: Issue title\n\
                     - body: Issue description\n\
                     - state: \"open\" or \"closed\"\n\
                     - labels: Array of label objects\n\
                     - assignees: Array of assigned users\n\
                     - created_at: Creation timestamp\n\
                     - updated_at: Last update timestamp\n\
                     - comments: Number of comments\n\
                     - html_url: Link to issue on GitHub\n\n\
                     Important notes:\n\
                     - issue_number is the issue number (e.g., #42), NOT the internal ID\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos, 'public_repo' for public\n\
                     - Works for both issues and pull requests (PRs are issues with pull_request field)",
                ),
            },
        ])
    }
}
