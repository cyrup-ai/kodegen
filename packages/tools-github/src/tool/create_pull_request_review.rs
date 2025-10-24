use anyhow;
use kodegen_mcp_schema::github::{CreatePullRequestReviewArgs, CreatePullRequestReviewPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use octocrab::models::pulls::ReviewAction;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

/// Tool for creating a review on a pull request
#[derive(Clone)]
pub struct CreatePullRequestReviewTool;

impl Tool for CreatePullRequestReviewTool {
    type Args = CreatePullRequestReviewArgs;
    type PromptArgs = CreatePullRequestReviewPromptArgs;

    fn name() -> &'static str {
        "create_pull_request_review"
    }

    fn description() -> &'static str {
        "Create a review on a pull request (approve, request changes, or comment). \
         Requires GITHUB_TOKEN environment variable with repo permissions."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        false // Multiple reviews can be submitted
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

        // Convert string event to ReviewAction enum
        let event = match args.event.to_uppercase().as_str() {
            "APPROVE" => ReviewAction::Approve,
            "REQUEST_CHANGES" => ReviewAction::RequestChanges,
            "COMMENT" => ReviewAction::Comment,
            _ => {
                return Err(McpError::InvalidArguments(format!(
                    "Invalid event '{}'. Must be APPROVE, REQUEST_CHANGES, or COMMENT",
                    args.event
                )));
            }
        };

        // Build options struct
        let options = crate::CreatePullRequestReviewOptions {
            event,
            body: args.body,
            commit_id: args.commit_id,
            comments: None, // Inline comments not supported in this tool
        };

        // Call API wrapper (returns AsyncTask<Result<Review, GitHubError>>)
        let task_result = client
            .create_pull_request_review(args.owner, args.repo, args.pull_number, options)
            .await;

        // Handle outer Result (channel error)
        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        // Handle inner Result (GitHub API error)
        let review =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        // Return serialized review
        Ok(serde_json::to_value(&review)?)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I approve a pull request?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use create_pull_request_review with event \"APPROVE\":\n\n\
                     create_pull_request_review({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"APPROVE\",\n\
                       \"body\": \"LGTM! Great work.\"\n\
                     })\n\n\
                     Event types:\n\
                     - \"APPROVE\" - Approve the PR (allows merging if required reviews are met)\n\
                     - \"REQUEST_CHANGES\" - Block PR until changes are made\n\
                     - \"COMMENT\" - Leave review comments without approval/blocking\n\n\
                     Optional fields:\n\
                     - body: Overall review comment (recommended for context)\n\
                     - commit_id: Specific commit SHA to review (defaults to latest)\n\n\
                     Examples:\n\n\
                     # Approve with comment\n\
                     create_pull_request_review({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"APPROVE\",\n\
                       \"body\": \"Looks good! All tests pass.\"\n\
                     })\n\n\
                     # Request changes\n\
                     create_pull_request_review({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"REQUEST_CHANGES\",\n\
                       \"body\": \"Please address the comments before merging.\"\n\
                     })\n\n\
                     # Comment only (no approval/block)\n\
                     create_pull_request_review({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"COMMENT\",\n\
                       \"body\": \"Some suggestions for improvement.\"\n\
                     })\n\n\
                     # Review specific commit\n\
                     create_pull_request_review({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"event\": \"APPROVE\",\n\
                       \"body\": \"This commit looks good.\",\n\
                       \"commit_id\": \"abc123...\"\n\
                     })\n\n\
                     Note: This creates a REVIEW, not individual line comments.\n\
                     Use add_pull_request_review_comment for inline code comments.\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos\n\
                     - User must have write access to the repository\n\
                     - For APPROVE: User must be authorized reviewer if required reviews are configured",
                ),
            },
        ])
    }
}
