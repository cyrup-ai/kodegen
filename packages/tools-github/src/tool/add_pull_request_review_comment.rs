use anyhow;
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool for adding inline review comments to a pull request
#[derive(Clone)]
pub struct AddPullRequestReviewCommentTool;

/// Arguments for `add_pull_request_review_comment` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddPullRequestReviewCommentArgs {
    /// Repository owner (user or organization)
    pub owner: String,

    /// Repository name
    pub repo: String,

    /// Pull request number
    pub pull_number: u64,

    /// Comment body text
    pub body: String,

    /// Commit SHA to comment on (required for new comments)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<String>,

    /// File path to comment on (required for new comments)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Line number in the diff to comment on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,

    /// Side of diff: "LEFT" or "RIGHT" (default: RIGHT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,

    /// Start line for multi-line comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,

    /// Side of start line: "LEFT" or "RIGHT"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_side: Option<String>,

    /// Subject type (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_type: Option<String>,

    /// Comment ID to reply to (for threaded replies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<u64>,
}

/// Prompt arguments for `add_pull_request_review_comment` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddPullRequestReviewCommentPromptArgs {}

impl Tool for AddPullRequestReviewCommentTool {
    type Args = AddPullRequestReviewCommentArgs;
    type PromptArgs = AddPullRequestReviewCommentPromptArgs;

    fn name() -> &'static str {
        "add_pull_request_review_comment"
    }

    fn description() -> &'static str {
        "Add an inline review comment to a pull request (comment on specific lines of code). \
         Supports single-line, multi-line, and threaded comments. Requires GITHUB_TOKEN."
    }

    fn read_only() -> bool {
        false // Creates data
    }

    fn destructive() -> bool {
        false // Doesn't delete anything
    }

    fn idempotent() -> bool {
        false // Multiple comments can be created
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

        // Build request
        let request = crate::github::AddPullRequestReviewCommentRequest {
            owner: args.owner,
            repo: args.repo,
            pr_number: args.pull_number,
            body: args.body,
            commit_id: args.commit_id,
            path: args.path,
            line: args.line,
            side: args.side,
            start_line: args.start_line,
            start_side: args.start_side,
            subject_type: args.subject_type,
            in_reply_to: args.in_reply_to,
        };

        // Call API wrapper (returns AsyncTask<Result<ReviewComment, GitHubError>>)
        let task_result = client.add_pull_request_review_comment(request).await;

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
                content: PromptMessageContent::text("How do I add an inline comment to a PR?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use add_pull_request_review_comment for inline code comments:\n\n\
                     # Simple inline comment on a line\n\
                     add_pull_request_review_comment({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"body\": \"Consider using const here instead of let\",\n\
                       \"commit_id\": \"abc123...\",\n\
                       \"path\": \"src/main.rs\",\n\
                       \"line\": 45,\n\
                       \"side\": \"RIGHT\"\n\
                     })\n\n\
                     # Multi-line comment (comment on lines 20-25)\n\
                     add_pull_request_review_comment({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"body\": \"This entire function could be simplified\",\n\
                       \"commit_id\": \"abc123...\",\n\
                       \"path\": \"src/utils.rs\",\n\
                       \"start_line\": 20,\n\
                       \"line\": 25,\n\
                       \"side\": \"RIGHT\"\n\
                     })\n\n\
                     # Reply to existing comment (threaded)\n\
                     add_pull_request_review_comment({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"body\": \"Good catch! I'll fix that.\",\n\
                       \"in_reply_to\": 123456789\n\
                     })\n\n\
                     # Comment on old code (LEFT side)\n\
                     add_pull_request_review_comment({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"pull_number\": 42,\n\
                       \"body\": \"This old implementation had a bug\",\n\
                       \"commit_id\": \"abc123...\",\n\
                       \"path\": \"src/legacy.rs\",\n\
                       \"line\": 30,\n\
                       \"side\": \"LEFT\"\n\
                     })\n\n\
                     Important parameters:\n\
                     - body: Comment text (supports Markdown)\n\
                     - commit_id: Get from PR details or latest commit (required for new comments)\n\
                     - path: Relative file path in repo (required for new comments)\n\
                     - line: Line number in the diff (required for new comments)\n\
                     - side: \"RIGHT\" for new code, \"LEFT\" for old code (default: RIGHT)\n\
                     - start_line: For multi-line comments, the starting line number\n\
                     - start_side: Side of the start line (LEFT or RIGHT)\n\
                     - in_reply_to: For threading replies to existing comments\n\n\
                     Comment types:\n\n\
                     1. **New inline comment**: Requires commit_id, path, line, and optionally side\n\
                     2. **Multi-line comment**: Requires commit_id, path, start_line, line, and optionally sides\n\
                     3. **Threaded reply**: Only requires in_reply_to (inherits position from parent)\n\n\
                     Tips:\n\
                     - Use RIGHT side for commenting on new/changed code (most common)\n\
                     - Use LEFT side for commenting on old code being removed\n\
                     - Multi-line comments span from start_line to line (inclusive)\n\
                     - Thread replies create conversations on specific code sections\n\
                     - Body supports Markdown: code blocks, links, mentions, etc.\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'repo' scope for private repos\n\
                     - User must have write access to the repository\n\
                     - For new comments: commit must be part of the PR\n\
                     - For replies: parent comment must exist",
                ),
            },
        ])
    }
}
