use anyhow;
use kodegen_mcp_schema::github::{RequestCopilotReviewArgs, RequestCopilotReviewPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

/// Tool for requesting GitHub Copilot to review a pull request
#[derive(Clone)]
pub struct RequestCopilotReviewTool;

impl Tool for RequestCopilotReviewTool {
    type Args = RequestCopilotReviewArgs;
    type PromptArgs = RequestCopilotReviewPromptArgs;

    fn name() -> &'static str {
        "request_copilot_review"
    }

    fn description() -> &'static str {
        "Request GitHub Copilot to review a pull request (experimental feature). \
         Triggers automated code review from Copilot. Requires GITHUB_TOKEN and Copilot access."
    }

    fn read_only() -> bool {
        false // Triggers an action
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        true // Can be called multiple times safely
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

        // Call API wrapper (returns AsyncTask<Result<(), GitHubError>>)
        let task_result = client
            .request_copilot_review(args.owner, args.repo, args.pull_number)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Return success message (convert unit to JSON)
        Ok(json!({
            "success": true,
            "message": "Copilot review requested successfully"
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I request a GitHub Copilot review?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use request_copilot_review to trigger Copilot PR analysis:\n\n\
                     request_copilot_review({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42\n\
                     })\n\n\
                     This is an EXPERIMENTAL feature that:\n\
                     - Requests GitHub Copilot to analyze the PR\n\
                     - Copilot will review code changes and provide suggestions\n\
                     - Results appear as PR comments/reviews\n\n\
                     What Copilot reviews:\n\
                     - Code quality and best practices\n\
                     - Potential bugs and issues\n\
                     - Security vulnerabilities\n\
                     - Performance improvements\n\
                     - Code style and conventions\n\n\
                     Requirements:\n\
                     - GitHub Copilot access on the repository\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos\n\
                     - User must have appropriate permissions\n\
                     - May not be available on all repository types\n\n\
                     Important notes:\n\
                     - This endpoint is EXPERIMENTAL and may change\n\
                     - The request triggers the review but doesn't return the review content\n\
                     - Check PR comments after a short delay to see Copilot's feedback\n\
                     - Review availability depends on repository settings\n\
                     - Not all repositories have Copilot review enabled\n\n\
                     Example workflow:\n\n\
                     1. Create or update a pull request\n\
                     2. Request Copilot review: request_copilot_review({...})\n\
                     3. Wait a few moments for Copilot to analyze\n\
                     4. Check PR comments: get_pull_request_reviews({...})\n\
                     5. Review Copilot's suggestions and feedback\n\n\
                     Use cases:\n\
                     - Automated code review for initial feedback\n\
                     - Catch common issues before human review\n\
                     - Get suggestions for improvements\n\
                     - Security and quality checks\n\
                     - Learn from AI-generated best practices\n\n\
                     Tip: Combine with get_pull_request_reviews to see all reviews\n\
                     including Copilot's automated feedback.",
                ),
            },
        ])
    }
}
