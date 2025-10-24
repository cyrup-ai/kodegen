use anyhow;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::ListCommitsArgs;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for listing repository commits
pub struct ListCommitsTool;

impl Tool for ListCommitsTool {
    type Args = ListCommitsArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "list_commits"
    }

    fn description() -> &'static str {
        "List commits in a repository with filtering options"
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
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            McpError::Other(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
        })?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {e}")))?;

        // Convert Args to ListCommitsOptions
        let options = crate::github::ListCommitsOptions {
            sha: args.sha,
            path: args.path,
            author: args.author,
            since: args.since,
            until: args.until,
            page: args.page,
            per_page: args.per_page,
        };

        let task_result = client.list_commits(args.owner, args.repo, options).await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let commits =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        Ok(serde_json::to_value(&commits)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub List Commits Examples

## List Recent Commits
To list the most recent commits from the default branch:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "per_page": 25
}
```

## Filter by Branch or SHA
To list commits from a specific branch:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "sha": "main",
  "per_page": 25
}
```

To start from a specific commit:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "sha": "abc123def456",
  "per_page": 25
}
```

## Filter by Author
To see commits from a specific author:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "author": "octocat",
  "per_page": 50
}
```

You can use either GitHub login or email address:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "author": "octocat@github.com"
}
```

## Filter by Date Range
To get commits within a specific time period:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "since": "2024-01-01T00:00:00Z",
  "until": "2024-12-31T23:59:59Z",
  "per_page": 100
}
```

To get commits after a certain date:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "since": "2024-06-01T00:00:00Z"
}
```

## Filter by File Path
To see commits that modified a specific file or directory:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "path": "src/main.rs"
}
```

For a directory:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "path": "src/components"
}
```

## Combine Multiple Filters
To get commits from a specific author on a specific branch within a date range:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "sha": "develop",
  "author": "octocat",
  "since": "2024-01-01T00:00:00Z",
  "path": "src/",
  "per_page": 50
}
```

## Common Use Cases

1. **Recent Activity**: List recent commits to see latest changes
2. **Author History**: Track contributions by specific developers
3. **File History**: See all changes to a specific file
4. **Release Notes**: Get commits between release dates
5. **Branch Comparison**: Compare commit history between branches
6. **Code Archaeology**: Find when specific code was introduced

## Response Information

Each commit object includes:
- **sha**: Unique commit identifier
- **commit**: Commit details (message, author, date, tree)
- **author**: GitHub user who authored the commit
- **committer**: GitHub user who committed (may differ from author)
- **parents**: Array of parent commit SHAs
- **html_url**: Web URL to view the commit

## Date Format

Use ISO 8601 format for since and until parameters:
- **Full**: `2024-01-15T14:30:00Z` (with time)
- **Date only**: `2024-01-15T00:00:00Z` (midnight UTC)
- **With timezone**: `2024-01-15T14:30:00-08:00`

## Pagination

- Default per_page is 30 commits
- Maximum per_page is 100
- Use page parameter for pagination
- Commits are returned in reverse chronological order (newest first)

## Best Practices

- Use specific branches when available (sha parameter)
- Combine filters to narrow results efficiently
- Use pagination for large result sets
- Filter by path to track file-specific history
- Use author filter to generate contributor reports
- Set appropriate date ranges to limit results
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
