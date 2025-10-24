//! Git worktree lock tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeLockArgs, GitWorktreeLockPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for locking worktrees
#[derive(Clone)]
pub struct GitWorktreeLockTool;

impl Tool for GitWorktreeLockTool {
    type Args = GitWorktreeLockArgs;
    type PromptArgs = GitWorktreeLockPromptArgs;

    fn name() -> &'static str {
        "git_worktree_lock"
    }

    fn description() -> &'static str {
        "Lock a worktree to prevent deletion. \
         Useful for worktrees on removable media or network drives."
    }

    fn read_only() -> bool {
        false // Writes lock file
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false // Fails if already locked
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build worktree lock options
        let mut opts = crate::WorktreeLockOpts::new(&args.worktree_path);
        if let Some(ref reason) = args.reason {
            opts = opts.reason(reason);
        }

        // Execute worktree lock
        crate::worktree_lock(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "worktree_path": args.worktree_path,
            "reason": args.reason,
            "message": "Worktree locked"
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
