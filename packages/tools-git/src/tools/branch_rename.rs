//! Git branch renaming tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitBranchRenameArgs, GitBranchRenamePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for renaming Git branches
#[derive(Clone)]
pub struct GitBranchRenameTool;

impl Tool for GitBranchRenameTool {
    type Args = GitBranchRenameArgs;
    type PromptArgs = GitBranchRenamePromptArgs;

    fn name() -> &'static str {
        "git_branch_rename"
    }

    fn description() -> &'static str {
        "Rename a branch in a Git repository. \
         Automatically updates HEAD if renaming the current branch."
    }

    fn read_only() -> bool {
        false // Modifies repository
    }

    fn destructive() -> bool {
        false // Renames, doesn't delete
    }

    fn idempotent() -> bool {
        false // Will fail if already renamed
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Rename branch
        crate::rename_branch(
            repo,
            args.old_name.clone(),
            args.new_name.clone(),
            args.force,
        )
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
        .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "old_name": args.old_name,
            "new_name": args.new_name,
            "message": format!("Renamed branch '{}' to '{}'", args.old_name, args.new_name)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
