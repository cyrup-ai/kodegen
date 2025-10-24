//! Git branch creation tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitBranchCreateArgs, GitBranchCreatePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for creating Git branches
#[derive(Clone)]
pub struct GitBranchCreateTool;

impl Tool for GitBranchCreateTool {
    type Args = GitBranchCreateArgs;
    type PromptArgs = GitBranchCreatePromptArgs;

    fn name() -> &'static str {
        "git_branch_create"
    }

    fn description() -> &'static str {
        "Create a new branch in a Git repository. \
         Optionally specify a starting point and checkout the branch after creation."
    }

    fn read_only() -> bool {
        false // Creates branches
    }

    fn destructive() -> bool {
        false // Only creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Will fail if branch exists without force
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build branch options
        let opts = crate::BranchOpts {
            name: args.branch.clone(),
            start_point: args.from_branch,
            force: args.force,
            checkout: args.checkout,
            track: false,
        };

        // Create branch
        crate::branch(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "branch": args.branch,
            "message": format!("Created branch '{}'", args.branch)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
