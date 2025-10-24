//! GitHub issue update tool

use anyhow;
use kodegen_mcp_schema::github::{UpdateIssueArgs, UpdateIssuePromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::github::UpdateIssueRequest;

/// Tool for updating GitHub issues
#[derive(Clone)]
pub struct UpdateIssueTool;

impl Tool for UpdateIssueTool {
    type Args = UpdateIssueArgs;
    type PromptArgs = UpdateIssuePromptArgs;

    fn name() -> &'static str {
        "update_issue"
    }

    fn description() -> &'static str {
        "Update an existing GitHub issue. Supports partial updates - only specified fields \
         will be modified. Can update title, body, state (open/closed), labels, and assignees. \
         Requires GITHUB_TOKEN environment variable with write access."
    }

    fn read_only() -> bool {
        false // Modifies data
    }

    fn destructive() -> bool {
        false // Modifies, doesn't delete
    }

    fn idempotent() -> bool {
        false // Multiple updates may differ
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

        // Convert state string to IssueState enum
        let state = args
            .state
            .as_ref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "open" => Some(octocrab::models::IssueState::Open),
                "closed" => Some(octocrab::models::IssueState::Closed),
                _ => None,
            });

        // Build request
        let request = UpdateIssueRequest {
            owner: args.owner,
            repo: args.repo,
            issue_number: args.issue_number,
            title: args.title,
            body: args.body,
            state,
            labels: args.labels,
            assignees: args.assignees,
            milestone: None,
        };

        // Call API wrapper (returns AsyncTask<Result<Issue, GitHubError>>)
        // The .await returns Result<Result<Issue, GitHubError>, RecvError>
        let task_result = client.update_issue(request).await;

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
                content: PromptMessageContent::text("How do I update a GitHub issue?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the update_issue tool to modify an existing GitHub issue:\n\n\
                     Close an issue:\n\
                     update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"state\": \"closed\"})\n\n\
                     Update title and body:\n\
                     update_issue({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"issue_number\": 42,\n\
                       \"title\": \"Updated: Bug in login\",\n\
                       \"body\": \"Revised description...\"\n\
                     })\n\n\
                     Replace labels:\n\
                     update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"labels\": [\"bug\", \"resolved\"]})\n\n\
                     Update assignees:\n\
                     update_issue({\"owner\": \"octocat\", \"repo\": \"hello-world\", \"issue_number\": 42, \"assignees\": [\"alice\", \"bob\"]})\n\n\
                     Combined update:\n\
                     update_issue({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"issue_number\": 42,\n\
                       \"state\": \"closed\",\n\
                       \"labels\": [\"bug\", \"fixed\"],\n\
                       \"body\": \"Fixed in PR #123\"\n\
                     })\n\n\
                     Important notes:\n\
                     - All fields are optional - only specified fields are updated\n\
                     - state: \"open\" or \"closed\"\n\
                     - labels: REPLACES all existing labels (not additive)\n\
                     - assignees: REPLACES all existing assignees (not additive)\n\
                     - To clear labels or assignees, pass empty array: []\n\
                     - Requires write access to the repository\n\
                     - GITHUB_TOKEN environment variable must be set",
                ),
            },
        ])
    }
}
