//! Git worktree unlock tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeUnlockArgs, GitWorktreeUnlockPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Tool for unlocking worktrees
#[derive(Clone)]
pub struct GitWorktreeUnlockTool;

impl Tool for GitWorktreeUnlockTool {
    type Args = GitWorktreeUnlockArgs;
    type PromptArgs = GitWorktreeUnlockPromptArgs;

    fn name() -> &'static str {
        "git_worktree_unlock"
    }

    fn description() -> &'static str {
        "Unlock a locked worktree. \
         Removes the lock that prevents worktree deletion."
    }

    fn read_only() -> bool {
        false // Removes lock file
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false // Fails if not locked
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Execute worktree unlock
        crate::worktree_unlock(repo, PathBuf::from(&args.worktree_path))
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "worktree_path": args.worktree_path,
            "message": "Worktree unlocked"
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
