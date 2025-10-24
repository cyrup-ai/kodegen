use anyhow;
use kodegen_mcp_schema::github::MergePullRequestArgs;
use kodegen_mcp_tool::{McpError, Tool};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for merging a pull request
pub struct MergePullRequestTool;

impl Tool for MergePullRequestTool {
    type Args = MergePullRequestArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "merge_pull_request"
    }

    fn description() -> &'static str {
        "Merge a pull request in a GitHub repository"
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true
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

        let options = crate::MergePullRequestOptions {
            commit_title: args.commit_title,
            commit_message: args.commit_message,
            sha: args.sha,
            merge_method: args.merge_method,
        };

        let task_result = client
            .merge_pull_request(args.owner, args.repo, args.pr_number, options)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let merge_result =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        Ok(serde_json::to_value(&merge_result)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Pull Request Merge Examples

## Basic Merge
To merge a pull request with default settings:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42
}
```

## Merge with Custom Commit Message
To merge with a custom commit title and message:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "commit_title": "Feature: Add user authentication",
  "commit_message": "This commit adds OAuth2 authentication support.\n\nCloses #123\nCloses #124"
}
```

## Squash Merge
To merge all commits into a single commit:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "merge_method": "squash",
  "commit_title": "Add authentication feature"
}
```

## Rebase Merge
To rebase and merge commits onto the base branch:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "merge_method": "rebase"
}
```

## Safe Merge with SHA Check
To ensure the PR hasn't changed since you last reviewed it:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42,
  "sha": "6dcb09b5b57875f334f61aebed695e2e4193db5e",
  "commit_title": "Merge feature after review"
}
```

## Merge Methods

- **merge** (default): Creates a merge commit, preserving all commits from the PR
- **squash**: Combines all commits into a single commit
- **rebase**: Rebases commits onto the base branch without a merge commit

## Common Use Cases

1. **Standard Merge**: Merge approved PRs with default settings
2. **Clean History**: Use squash merge for feature branches with many small commits
3. **Linear History**: Use rebase merge to maintain a linear commit history
4. **Custom Messages**: Provide detailed commit messages for important merges
5. **Safe Merging**: Use SHA verification to prevent merging outdated code

## Best Practices

- **Review First**: Always review and approve PRs before merging
- **Check CI**: Ensure all checks pass before merging
- **Choose Method**: Select merge method based on project conventions
- **Update Message**: Provide clear commit messages, especially for squash merges
- **Use SHA Check**: For critical merges, verify the exact commit being merged
- **Clean Up**: Delete the branch after merging (done automatically in many repos)

## Safety Notes

- This is a **destructive operation** - merged code becomes part of the base branch
- Cannot be easily undone (requires revert commits)
- Ensure proper testing and review before merging
- Use SHA parameter to prevent race conditions
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
