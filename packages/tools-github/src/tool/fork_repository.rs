use anyhow;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::ForkRepositoryArgs;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for forking a repository
pub struct ForkRepositoryTool;

impl Tool for ForkRepositoryTool {
    type Args = ForkRepositoryArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "fork_repository"
    }

    fn description() -> &'static str {
        "Fork a repository to your account or an organization"
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false
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
            .fork_repository(args.owner, args.repo, args.organization)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let repository =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        Ok(serde_json::to_value(&repository)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Repository Fork Examples

## Fork to Your Account
To fork a repository to your personal account:

```json
{
  "owner": "octocat",
  "repo": "hello-world"
}
```

## Fork to Organization
To fork a repository to an organization you belong to:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "organization": "my-org"
}
```

## Common Use Cases

1. **Contributing**: Fork a project to make contributions via pull requests
2. **Experimentation**: Fork to try changes without affecting the original
3. **Starting Point**: Fork to use as a template for your own project
4. **Organization Copy**: Fork to your organization for internal use

## What is a Fork?

A fork is a complete copy of a repository that:
- Lives under your account or organization
- Maintains a connection to the original repository
- Allows you to freely experiment with changes
- Enables contributing back via pull requests
- Includes all branches, commits, and history

## Workflow After Forking

1. **Fork** the repository (this tool)
2. **Clone** your fork to your local machine
3. **Create** a new branch for your changes
4. **Make** your changes and commit them
5. **Push** to your fork
6. **Create** a pull request to the original repository

## Best Practices

- Fork when you plan to contribute back to the project
- Keep your fork synced with the upstream repository
- Use descriptive branch names for your changes
- Follow the project's contribution guidelines
- Test your changes before creating pull requests

## Important Notes

- Forking is instantaneous but may take a few moments for large repositories
- You cannot fork your own repositories
- You cannot fork a repository you've already forked (delete the old fork first)
- Forks maintain a link to the upstream (original) repository
- You can configure whether to fork all branches or just the default branch

## Response Information

The response includes:
- **id**: Unique repository ID of the fork
- **full_name**: Your username or org/repo format
- **html_url**: Web URL to your forked repository
- **clone_url**: HTTPS clone URL for your fork
- **fork**: true (indicates this is a fork)
- **parent**: Information about the original repository
- **source**: Information about the root repository (if parent is also a fork)
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
