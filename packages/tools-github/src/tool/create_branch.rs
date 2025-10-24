use anyhow;
use kodegen_mcp_tool::{McpError, Tool};
use kodegen_mcp_schema::github::CreateBranchArgs;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;

use crate::GitHubClient;

/// Tool for creating a new branch
pub struct CreateBranchTool;

impl Tool for CreateBranchTool {
    type Args = CreateBranchArgs;
    type PromptArgs = ();

    fn name() -> &'static str {
        "create_branch"
    }

    fn description() -> &'static str {
        "Create a new branch from a commit SHA"
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
            .create_branch(args.owner, args.repo, args.branch_name, args.sha)
            .await;

        let api_result =
            task_result.map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {e}")))?;

        let reference =
            api_result.map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {e}")))?;

        Ok(serde_json::to_value(&reference)?)
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::text(
                r#"# GitHub Create Branch Examples

## Create Branch from Specific SHA
To create a new branch from a specific commit:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "branch_name": "feature/new-feature",
  "sha": "abc123def456789abc123def456789abc123def4"
}
```

## Create Branch from Latest Commit

### Step 1: Get the latest commit SHA
First, use `list_commits` to get recent commits:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "sha": "main",
  "per_page": 1
}
```

### Step 2: Create branch from that SHA
Use the SHA from the response:

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "branch_name": "feature/from-main",
  "sha": "abc123def456"
}
```

## How to Get SHA from Commit History

### Method 1: List Commits
Use `list_commits` to see commit history:
- Returns array of commits with SHA, message, author, date
- Can filter by branch, author, date range, or file path
- Most recent commits appear first

### Method 2: Get Specific Commit
Use `get_commit` if you know the commit identifier:
- Provides detailed information including full SHA
- Shows files changed and diff stats

### Method 3: From Branch Info
Use `list_branches` to get branch tip SHA:
- Each branch object includes latest commit SHA
- Useful for creating branches from other branches

## Branch Naming Best Practices

**Feature Branches:**
- `feature/user-authentication`
- `feature/payment-integration`

**Bug Fixes:**
- `bugfix/login-error`
- `fix/memory-leak`

**Hotfixes:**
- `hotfix/security-patch`
- `hotfix/critical-bug`

**Release Branches:**
- `release/v1.0.0`
- `release/2024-01`

## Common Workflows

### 1. Feature Development
```
1. Get main branch SHA
2. Create feature branch from main
3. Develop feature
4. Create pull request back to main
```

### 2. Release Preparation
```
1. Get develop branch SHA
2. Create release branch
3. Make final adjustments
4. Merge to main and tag
```

### 3. Hotfix
```
1. Get production tag SHA
2. Create hotfix branch
3. Fix critical issue
4. Merge to main and develop
```

## Important Notes

- **Branch Name Uniqueness**: Cannot create a branch with an existing name
- **SHA Format**: Must be a valid 40-character commit SHA
- **Permissions**: Requires write access to the repository
- **Protected Branches**: Cannot be created via API if name matches protection rules
- **Immediate Availability**: Branch is available immediately after creation

## Response Information

The response includes:
- **ref**: Full reference name (refs/heads/branch-name)
- **node_id**: GitHub's internal node identifier
- **url**: API URL for the reference
- **object**: Object containing the SHA and type (commit)

## Troubleshooting

- **"Reference already exists"**: Branch name is already taken
- **"Object does not exist"**: SHA is invalid or not found
- **"Not Found"**: Repository doesn't exist or no write access
"#,
            ),
        }])
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
