//! Git worktree prune tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreePruneArgs, GitWorktreePrunePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for pruning stale worktrees
#[derive(Clone)]
pub struct GitWorktreePruneTool;

impl Tool for GitWorktreePruneTool {
    type Args = GitWorktreePruneArgs;
    type PromptArgs = GitWorktreePrunePromptArgs;

    fn name() -> &'static str {
        "git_worktree_prune"
    }

    fn description() -> &'static str {
        "Remove stale worktree administrative files. \
         Cleans up .git/worktrees/ entries for worktrees whose directories have been manually deleted. \
         Returns list of pruned worktree names."
    }

    fn read_only() -> bool {
        false // Removes stale admin files
    }

    fn destructive() -> bool {
        true // Deletes worktree admin
    }

    fn idempotent() -> bool {
        true // Safe to call repeatedly
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Execute worktree prune
        let pruned = crate::worktree_prune(repo)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "pruned": pruned,
            "count": pruned.len(),
            "message": format!("Pruned {} stale worktree(s)", pruned.len())
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
