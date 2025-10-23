//! Git worktree lock tool

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for locking worktrees
#[derive(Clone)]
pub struct GitWorktreeLockTool;

/// Arguments for `git_worktree_lock` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreeLockArgs {
    /// Path to repository
    pub path: String,

    /// Path to the worktree to lock (prevents deletion)
    pub worktree_path: String,

    /// Optional reason for locking (e.g., "On removable drive").
    /// Stored in the lock file for documentation purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Prompt arguments for `git_worktree_lock` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreeLockPromptArgs {}

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
