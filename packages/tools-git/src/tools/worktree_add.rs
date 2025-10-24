//! Git worktree add tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitWorktreeAddArgs, GitWorktreeAddPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for adding worktrees
#[derive(Clone)]
pub struct GitWorktreeAddTool;

impl Tool for GitWorktreeAddTool {
    type Args = GitWorktreeAddArgs;
    type PromptArgs = GitWorktreeAddPromptArgs;

    fn name() -> &'static str {
        "git_worktree_add"
    }

    fn description() -> &'static str {
        "Create a new worktree linked to the repository. \
         Allows working on multiple branches simultaneously."
    }

    fn read_only() -> bool {
        false // Creates worktree
    }

    fn destructive() -> bool {
        false // Creates new files
    }

    fn idempotent() -> bool {
        false // Fails if worktree exists
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build worktree add options
        let mut opts = crate::WorktreeAddOpts::new(&args.worktree_path);
        if let Some(ref branch) = args.branch {
            opts = opts.committish(branch);
        }
        opts = opts.force(args.force);

        // Execute worktree add
        let created_path = crate::worktree_add(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "worktree_path": created_path.display().to_string(),
            "branch": args.branch
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
