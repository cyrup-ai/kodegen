//! Git worktree remove tool

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for removing worktrees
#[derive(Clone)]
pub struct GitWorktreeRemoveTool;

/// Arguments for `git_worktree_remove` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreeRemoveArgs {
    /// Path to repository
    pub path: String,

    /// Path to the worktree to remove (both working directory and admin files)
    pub worktree_path: String,

    /// Force removal even if worktree is locked (default: false)
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_worktree_remove` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreeRemovePromptArgs {}

impl Tool for GitWorktreeRemoveTool {
    type Args = GitWorktreeRemoveArgs;
    type PromptArgs = GitWorktreeRemovePromptArgs;

    fn name() -> &'static str {
        "git_worktree_remove"
    }

    fn description() -> &'static str {
        "Remove a worktree and its associated administrative files. \
         Cannot remove locked worktrees without force flag."
    }

    fn read_only() -> bool {
        false // Deletes files
    }

    fn destructive() -> bool {
        true // Removes worktree and files
    }

    fn idempotent() -> bool {
        false // Fails if worktree doesn't exist
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build worktree remove options
        let opts = crate::WorktreeRemoveOpts::new(&args.worktree_path).force(args.force);

        // Execute worktree remove
        crate::worktree_remove(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "worktree_path": args.worktree_path,
            "message": "Worktree removed"
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
