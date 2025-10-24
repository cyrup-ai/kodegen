use anyhow;
use kodegen_mcp_schema::github::{GetPullRequestReviewsArgs, GetPullRequestReviewsPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use tokio_stream::StreamExt;

/// Tool for getting all reviews for a pull request
#[derive(Clone)]
pub struct GetPullRequestReviewsTool;

impl Tool for GetPullRequestReviewsTool {
    type Args = GetPullRequestReviewsArgs;
    type PromptArgs = GetPullRequestReviewsPromptArgs;

    fn name() -> &'static str {
        "get_pull_request_reviews"
    }

    fn description() -> &'static str {
        "Get all reviews for a pull request. Shows approval status, requested changes, \
         and comments from reviewers. Requires GITHUB_TOKEN environment variable."
    }

    fn read_only() -> bool {
        true // Only reads data
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        true // Same result every time
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

        // Call API wrapper (returns AsyncStream<Result<Review, GitHubError>>)
        let mut review_stream =
            client.get_pull_request_reviews(args.owner, args.repo, args.pull_number);

        // Collect stream into vector
        let mut reviews = Vec::new();
        while let Some(result) = review_stream.next().await {
            let review =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            reviews.push(review);
        }

        // Return serialized reviews
        Ok(json!({
            "reviews": reviews,
            "count": reviews.len()
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I see all reviews on a pull request?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use get_pull_request_reviews to see all reviews:\n\n\
                     get_pull_request_reviews({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42\n\
                     })\n\n\
                     Returns array of reviews with:\n\
                     - id: Review ID\n\
                     - user: Reviewer username and profile\n\
                     - body: Review comment text\n\
                     - state: \"APPROVED\", \"CHANGES_REQUESTED\", \"COMMENTED\", \"DISMISSED\", \"PENDING\"\n\
                     - submitted_at: When review was submitted\n\
                     - commit_id: SHA the review is associated with\n\n\
                     Each review shows:\n\
                     - Whether the reviewer approved, requested changes, or just commented\n\
                     - Any overall review comments\n\
                     - When it was submitted\n\
                     - Which commit was reviewed\n\n\
                     Use this to:\n\
                     - Check approval status before merging\n\
                     - See who has reviewed and their feedback\n\
                     - Understand what changes were requested\n\
                     - Track review history over time\n\n\
                     Review states:\n\
                     - APPROVED: Reviewer approved the changes\n\
                     - CHANGES_REQUESTED: Reviewer wants changes before approval\n\
                     - COMMENTED: Reviewer left comments without approval/blocking\n\
                     - DISMISSED: Review was dismissed (no longer valid)\n\
                     - PENDING: Review is in progress but not submitted\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos\n\
                     - User must have read access to the repository",
                ),
            },
        ])
    }
}
