//! Git branch deletion tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitBranchDeleteArgs, GitBranchDeletePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for deleting Git branches
#[derive(Clone)]
pub struct GitBranchDeleteTool;

impl Tool for GitBranchDeleteTool {
    type Args = GitBranchDeleteArgs;
    type PromptArgs = GitBranchDeletePromptArgs;

    fn name() -> &'static str {
        "git_branch_delete"
    }

    fn description() -> &'static str {
        "Delete a branch from a Git repository. \
         Cannot delete the currently checked-out branch."
    }

    fn read_only() -> bool {
        false // Modifies repository
    }

    fn destructive() -> bool {
        true // Deletes branches
    }

    fn idempotent() -> bool {
        false // Will fail if branch doesn't exist
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Delete branch
        crate::delete_branch(repo, args.branch.clone(), args.force)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "branch": args.branch,
            "message": format!("Deleted branch '{}'", args.branch)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
