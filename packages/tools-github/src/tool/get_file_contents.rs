use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::GetFileContentsArgs;
use serde_json::Value;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use anyhow;

use crate::GitHubClient;

/// Tool for getting file or directory contents from a GitHub repository
pub struct GetFileContentsTool;

impl Tool for GetFileContentsTool {
    type Args = GetFileContentsArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "get_file_contents"
    }

    fn description() -> &'static str {
        "Get file or directory contents from a GitHub repository"
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
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| McpError::Other(anyhow::anyhow!(
                "GITHUB_TOKEN environment variable not set"
            )))?;

        let client = GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {}", e)))?;

        let task_result = client
            .get_file_contents(
                args.owner,
                args.repo,
                args.path,
                args.ref_name,
            )
            .await;

        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;

        let contents = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;

        Ok(serde_json::to_value(&contents)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub File Contents Examples

## Get a Single File
To retrieve the contents of a file from a repository:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "path": "README.md"
}
```

## Get File from Specific Branch
To get a file from a specific branch, tag, or commit:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "path": "src/main.rs",
  "ref_name": "develop"
}
```

## Get File from Specific Commit
To retrieve a file at a specific commit SHA:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "path": "config.json",
  "ref_name": "a1b2c3d4e5f6"
}
```

## List Directory Contents
To list all files and subdirectories in a directory:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "path": "src"
}
```

## Response Information

For **files**, the response includes:
- **name**: File name
- **path**: Full path in repository
- **type**: "file"
- **content**: Base64-encoded file content
- **size**: File size in bytes
- **sha**: Git blob SHA
- **download_url**: Direct download URL
- **html_url**: GitHub web URL

For **directories**, the response is an array of items, each with:
- **name**: Item name
- **path**: Full path
- **type**: "file" or "dir"
- **size**: Size (for files)
- **sha**: Git SHA
- **html_url**: GitHub web URL

## Decoding File Content

File content is base64-encoded. To decode:

**JavaScript:**
```javascript
const content = Buffer.from(base64Content, 'base64').toString('utf-8');
```

**Python:**
```python
import base64
content = base64.b64decode(base64_content).decode('utf-8')
```

**Rust:**
```rust
use base64::{Engine as _, engine::general_purpose};
let content = general_purpose::STANDARD.decode(base64_content)?;
let text = String::from_utf8(content)?;
```

## Common Use Cases

1. **Read Configuration**: Get config files for analysis
2. **Code Review**: Fetch source files for review
3. **Documentation**: Retrieve README and docs
4. **Directory Browsing**: Navigate repository structure
5. **Content Analysis**: Analyze file contents programmatically
6. **Historical Versions**: Get files from specific commits

## Best Practices

- Use `ref_name` to ensure you're reading from the correct branch
- For directories, the API returns all items (may be limited for very large directories)
- Check the `type` field to distinguish files from directories
- Cache frequently accessed files to reduce API calls
- Use `sha` to detect if file has changed between requests

## Error Handling

- **404**: File or directory not found at the specified path
- **403**: Access denied (private repository, no permissions)
- **401**: Invalid or missing GITHUB_TOKEN
- **422**: Invalid reference name
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
