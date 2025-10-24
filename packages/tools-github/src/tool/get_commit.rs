use anyhow;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::GetCommitArgs;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for getting detailed commit information
pub struct GetCommitTool;

impl Tool for GetCommitTool {
    type Args = GetCommitArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "get_commit"
    }

    fn description() -> &'static str {
        "Get detailed information about a specific commit"
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

        let task_result = client
            .get_commit(
                args.owner,
                args.repo,
                args.commit_sha,
                args.page,
                args.per_page,
            )
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let commit =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        Ok(serde_json::to_value(&commit)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Get Commit Examples

## Get Commit Details
To get detailed information about a specific commit:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "commit_sha": "abc123def456789abc123def456789abc123def4"
}
```

## Get Commit with Pagination for Files
For commits with many changed files, use pagination:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "commit_sha": "abc123def456",
  "page": 1,
  "per_page": 100
}
```

## Response Information

The response includes comprehensive commit details:

**Basic Information:**
- **sha**: Full commit SHA
- **commit**: Commit object with message, author, committer, tree
- **author**: GitHub user object (may be null for external commits)
- **committer**: GitHub user object
- **parents**: Array of parent commit SHAs
- **html_url**: Web URL to view the commit

**Change Statistics:**
- **stats**: Object with total additions, deletions, and changes
- **files**: Array of changed files with patches

**File Details (for each file):**
- **filename**: Path to the file
- **status**: Change type (added, modified, removed, renamed)
- **additions**: Lines added
- **deletions**: Lines deleted
- **changes**: Total changes
- **patch**: The actual diff content (if available)

## Common Use Cases

1. **Code Review**: Examine specific commit changes in detail
2. **Debugging**: Investigate when and how a bug was introduced
3. **Audit Trail**: Review security-sensitive changes
4. **Documentation**: Generate change logs with detailed diffs
5. **Analysis**: Calculate code churn metrics
6. **Verification**: Confirm specific changes were made
7. **Integration**: Trigger workflows based on commit content

## Understanding Commit SHAs

**Full SHA:**
- 40 hexadecimal characters
- Example: `abc123def456789abc123def456789abc123def4`
- Uniquely identifies a commit

**Short SHA:**
- First 7-10 characters
- Example: `abc123d`
- Can be used in many GitHub APIs
- This tool accepts both full and short SHAs

**Getting SHAs:**
- Use `list_commits` to get recent commit SHAs
- From PR file changes in pull request APIs
- From branch information in `list_branches`
- From GitHub web UI commit history

## Working with Diffs

The patch field contains standard unified diff format:
- Lines starting with `-` are removed
- Lines starting with `+` are added
- Lines starting with `@@` show line numbers
- Context lines show surrounding code

## Pagination for Large Commits

Some commits change many files:
- Use page and per_page to paginate through files
- Default is 30 files per page
- Maximum is 100 files per page
- Useful for merge commits or large refactorings

## Best Practices

- Cache commit information to avoid repeated API calls
- Use short SHAs when displaying to users
- Check the stats object for commit size before processing files
- Handle null author/committer (can occur for old or external commits)
- Be aware of rate limits when fetching many commits
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
