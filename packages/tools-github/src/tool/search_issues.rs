//! GitHub issues search tool

use anyhow;
use futures::StreamExt;
use kodegen_mcp_schema::github::{SearchIssuesArgs, SearchIssuesPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

/// Tool for searching GitHub issues using GitHub's search syntax
#[derive(Clone)]
pub struct SearchIssuesTool;

impl Tool for SearchIssuesTool {
    type Args = SearchIssuesArgs;
    type PromptArgs = SearchIssuesPromptArgs;

    fn name() -> &'static str {
        "search_issues"
    }

    fn description() -> &'static str {
        "Search for issues across GitHub using GitHub's powerful search syntax. \
         Supports filtering by repository, state, labels, assignee, author, dates, and more. \
         Returns matching issues with relevance ranking. \
         Requires GITHUB_TOKEN environment variable. Note: Search API has stricter rate limits."
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

        // Convert per_page to u8 (GitHub API expects u8)
        let per_page = args.per_page.map(|p| p.min(100) as u8);

        // Call API wrapper
        let mut issue_stream =
            client.search_issues(args.query, args.sort, args.order, args.page, per_page);

        // Collect stream results
        let mut issues = Vec::new();
        while let Some(result) = issue_stream.next().await {
            let issue =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            issues.push(issue);
        }

        // Return serialized issues
        Ok(json!({ "issues": issues, "count": issues.len() }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I search for GitHub issues using the search_issues tool?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The search_issues tool uses GitHub's powerful search syntax. Here are comprehensive examples:\n\n\
                     BASIC SEARCHES:\n\
                     Search in specific repo:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:open\"})\n\n\
                     Search by state:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:closed\"})\n\n\
                     FILTER BY LABELS:\n\
                     Single label:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world label:bug\"})\n\
                     Multiple labels (AND):\n\
                     search_issues({\"query\": \"repo:octocat/hello-world label:bug label:priority-high\"})\n\n\
                     FILTER BY PEOPLE:\n\
                     By assignee:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world assignee:octocat\"})\n\
                     By author:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world author:alice\"})\n\
                     By participant:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world involves:bob\"})\n\n\
                     DATE FILTERS:\n\
                     Created after date:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world created:>=2024-01-01\"})\n\
                     Updated recently:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world updated:>=2024-03-01\"})\n\
                     Date range:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world created:2024-01-01..2024-12-31\"})\n\n\
                     TEXT SEARCH:\n\
                     In title or body:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world authentication error\"})\n\
                     In title only:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world authentication in:title\"})\n\
                     In body only:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world error in:body\"})\n\n\
                     COMBINED FILTERS:\n\
                     Complex query:\n\
                     search_issues({\n\
                       \"query\": \"repo:octocat/hello-world is:open label:bug assignee:alice created:>=2024-01-01\",\n\
                       \"sort\": \"created\",\n\
                       \"order\": \"desc\"\n\
                     })\n\n\
                     SORTING:\n\
                     - sort: \"created\", \"updated\", \"comments\", \"reactions\"\n\
                     - order: \"asc\" (ascending) or \"desc\" (descending)\n\n\
                     PAGINATION:\n\
                     search_issues({\"query\": \"repo:octocat/hello-world is:open\", \"per_page\": 50, \"page\": 2})\n\n\
                     IMPORTANT NOTES:\n\
                     - Search API has stricter rate limits (30 requests/minute authenticated)\n\
                     - Results are relevance-ranked by default\n\
                     - Use repo:owner/name to search specific repository\n\
                     - Combine multiple filters with spaces\n\
                     - Date format: YYYY-MM-DD\n\
                     - Use quotes for multi-word searches: \"bug report\"\n\
                     - GITHUB_TOKEN environment variable must be set",
                ),
            },
        ])
    }
}
