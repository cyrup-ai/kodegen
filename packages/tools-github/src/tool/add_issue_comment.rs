//! GitHub issue comment addition tool

use anyhow;
use kodegen_mcp_schema::github::{AddIssueCommentArgs, AddIssueCommentPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

/// Tool for adding comments to GitHub issues
#[derive(Clone)]
pub struct AddIssueCommentTool;

impl Tool for AddIssueCommentTool {
    type Args = AddIssueCommentArgs;
    type PromptArgs = AddIssueCommentPromptArgs;

    fn name() -> &'static str {
        "add_issue_comment"
    }

    fn description() -> &'static str {
        "Add a comment to an existing GitHub issue. Supports Markdown formatting in the comment body. \
         Requires GITHUB_TOKEN environment variable with write access to the repository."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Creates new comment each time
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

        // Call API wrapper (returns AsyncTask<Result<Comment, GitHubError>>)
        // The .await returns Result<Result<Comment, GitHubError>, RecvError>
        let task_result = client
            .add_issue_comment(args.owner, args.repo, args.issue_number, args.body)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let comment =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Return serialized comment
        Ok(serde_json::to_value(&comment)?)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I add a comment to a GitHub issue?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use the add_issue_comment tool to add a comment to an existing issue:\n\n\
                     Basic usage:\n\
                     add_issue_comment({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"issue_number\": 42,\n\
                       \"body\": \"This has been fixed in the latest release.\"\n\
                     })\n\n\
                     With Markdown formatting:\n\
                     add_issue_comment({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"issue_number\": 42,\n\
                       \"body\": \"Fixed in PR #123\\n\\n```python\\nprint('hello')\\n```\"\n\
                     })\n\n\
                     With @mentions:\n\
                     add_issue_comment({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"issue_number\": 42,\n\
                       \"body\": \"@octocat Can you review this fix?\"\n\
                     })\n\n\
                     Features:\n\
                     - Full Markdown support (headings, code blocks, lists, etc.)\n\
                     - @mention users to notify them\n\
                     - Reference other issues/PRs with #number\n\
                     - Link commits with SHA hashes\n\
                     - Add emojis with :emoji_name:\n\n\
                     Important notes:\n\
                     - This tool CREATES a new comment each time (not idempotent)\n\
                     - Cannot edit existing comments (separate tool needed)\n\
                     - Requires write access to the repository\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Works for both issues and pull requests",
                ),
            },
        ])
    }
}
