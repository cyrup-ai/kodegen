use anyhow;
use futures::StreamExt;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::GetPullRequestFilesArgs;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

use crate::GitHubClient;

/// Tool for getting all files changed in a pull request
pub struct GetPullRequestFilesTool;

impl Tool for GetPullRequestFilesTool {
    type Args = GetPullRequestFilesArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "get_pull_request_files"
    }

    fn description() -> &'static str {
        "Get all files changed in a pull request with their diff stats"
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

        let mut file_stream = client.get_pull_request_files(args.owner, args.repo, args.pr_number);

        let mut files = Vec::new();
        while let Some(result) = file_stream.next().await {
            let file =
                result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;
            files.push(file);
        }

        Ok(json!({
            "files": files,
            "count": files.len()
        }))
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Pull Request Files Examples

## Get Changed Files
To retrieve all files changed in a pull request:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42
}
```

## Response Information

The response includes detailed information for each changed file:

- **filename**: Path to the file
- **status**: Change type (added, modified, removed, renamed)
- **additions**: Number of lines added
- **deletions**: Number of lines deleted
- **changes**: Total number of changes (additions + deletions)
- **patch**: The actual diff/patch content (if available)
- **blob_url**: URL to view the file at this version
- **raw_url**: URL to download the raw file
- **previous_filename**: Original filename (for renamed files)

## Common Use Cases

1. **Code Review**: See all files that need review
2. **Impact Analysis**: Assess the scope of changes
3. **Automated Checks**: Build tools that analyze changed files
4. **Documentation Updates**: Identify if docs need updates based on code changes
5. **Test Coverage**: Determine what tests might be needed
6. **Conflict Detection**: Identify files likely to have conflicts
7. **File Type Analysis**: Check if specific file types were modified

## Example Workflows

### Review Preparation
```
1. Get PR files
2. Filter by file extension (e.g., .rs, .js, .py)
3. Sort by changes count to prioritize review
4. Check if test files were added/modified
```

### Automated Analysis
```
1. Get PR files
2. Check if specific critical files were modified
3. Verify that tests were added for new code
4. Ensure documentation was updated
5. Run custom linters on changed files
```

### Change Summary
```
1. Get PR files
2. Group by directory
3. Calculate total additions/deletions
4. Identify largest changes
5. Generate summary report
```

## File Status Types

- **added**: New file created
- **modified**: Existing file changed
- **removed**: File deleted
- **renamed**: File moved or renamed
- **copied**: File copied from another

## Best Practices

- Use to assess PR size and complexity
- Check that appropriate files were modified (e.g., tests with code)
- Identify if changes touch critical infrastructure
- Build automated review workflows
- Track which files change frequently
- Ensure naming conventions are followed
- Verify file organization standards

## Response Format

```json
{
  "files": [
    {
      "filename": "src/main.rs",
      "status": "modified",
      "additions": 15,
      "deletions": 3,
      "changes": 18,
      "patch": "@@ -10,7 +10,19 @@...",
      ...
    }
  ],
  "count": 5
}
```
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
