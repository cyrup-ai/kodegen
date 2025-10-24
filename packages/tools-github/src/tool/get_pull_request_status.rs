use anyhow;
use kodegen_mcp_schema::github::GetPullRequestStatusArgs;
use kodegen_mcp_tool::{McpError, Tool};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for getting detailed status information about a pull request
pub struct GetPullRequestStatusTool;

impl Tool for GetPullRequestStatusTool {
    type Args = GetPullRequestStatusArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "get_pull_request_status"
    }

    fn description() -> &'static str {
        "Get detailed status information about a pull request including merge status, checks, and review state"
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
            .get_pull_request_status(args.owner, args.repo, args.pr_number)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let status =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        Ok(serde_json::to_value(&status)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Pull Request Status Examples

## Get Pull Request Status
To check the status of a pull request:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "pr_number": 42
}
```

## Response Information

The response includes comprehensive status information:

- **Basic Info**: PR number, title, state (open/closed), author
- **Merge Status**: Whether the PR can be merged, merge conflicts
- **Base/Head**: Target branch and source branch information
- **Checks**: CI/CD status, required checks, test results
- **Reviews**: Review state (approved, changes requested, pending)
- **Labels**: Labels applied to the PR
- **Assignees**: Assigned reviewers and assignees
- **Draft Status**: Whether the PR is marked as a draft
- **Mergeable State**: Detailed merge status (clean, dirty, blocked, etc.)

## Common Use Cases

1. **Pre-merge Check**: Verify PR is ready to merge before attempting merge
2. **CI Monitoring**: Check if all required checks have passed
3. **Review Status**: See if PR has required approvals
4. **Conflict Detection**: Identify if there are merge conflicts
5. **Workflow Automation**: Use in automation scripts to make decisions
6. **Status Dashboards**: Build tools that monitor PR status across repositories

## Example Workflow

```
1. Get PR status
2. Check if mergeable_state is "clean" or "unstable"
3. Verify all required checks passed
4. Confirm sufficient approvals
5. Proceed with merge if all conditions met
```

## Status Fields to Check

- **state**: "open" or "closed"
- **mergeable**: true/false/null (null means GitHub is still calculating)
- **mergeable_state**: "clean", "dirty", "blocked", "unstable", etc.
- **draft**: true if PR is still a draft
- **merged**: true if already merged
- **reviews**: Array of review states

## Best Practices

- Check status before attempting automated merges
- Monitor check runs and their conclusions
- Verify review requirements are met
- Handle null mergeable state (GitHub still calculating)
- Use for building PR dashboards and monitoring tools
- Implement retry logic if mergeable is null
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
