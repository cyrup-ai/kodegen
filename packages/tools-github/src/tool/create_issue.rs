//! GitHub issue creation tool

use anyhow;
use kodegen_mcp_schema::github::{CreateIssueArgs, CreateIssuePromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

/// Tool for creating GitHub issues
#[derive(Clone)]
pub struct CreateIssueTool;

impl Tool for CreateIssueTool {
    type Args = CreateIssueArgs;
    type PromptArgs = CreateIssuePromptArgs;

    fn name() -> &'static str {
        "create_issue"
    }

    fn description() -> &'static str {
        "Create a new issue in a GitHub repository. Supports setting title, body, \
         labels, and assignees. Requires GITHUB_TOKEN environment variable with appropriate permissions."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Multiple calls create multiple issues
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
            .create_issue(
                args.owner,
                args.repo,
                args.title,
                args.body,
                args.assignees,
                args.labels,
            )
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
                content: PromptMessageContent::text(
                    "How do I create a GitHub issue with labels and assignees?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the create_issue tool to create a GitHub issue:\n\n\
                     Basic usage:\n\
                     create_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"title\": \"Bug report\"})\n\n\
                     With body and labels:\n\
                     create_issue({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"title\": \"Bug: Login fails\",\n\
                       \"body\": \"When I try to login, the form doesn't submit...\",\n\
                       \"labels\": [\"bug\", \"priority-high\"],\n\
                       \"assignees\": [\"octocat\"]\n\
                     })\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos, 'public_repo' for public\n\
                     - User must have write access to the repository\n\
                     - Labels must already exist in the repository\n\
                     - Assignees must be collaborators on the repository\n\n\
                     Tips:\n\
                     - Body supports Markdown formatting\n\
                     - You can @mention users in the body\n\
                     - Labels are case-sensitive\n\
                     - Multiple assignees can be specified",
                ),
            },
        ])
    }
}
