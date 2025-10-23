//! Git worktree add tool

use kodegen_mcp_tool::{Tool, error::McpError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::{json, Value};
use rmcp::model::{PromptArgument, PromptMessage};
use std::path::Path;

/// Tool for adding worktrees
#[derive(Clone)]
pub struct GitWorktreeAddTool;

/// Arguments for `git_worktree_add` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitWorktreeAddArgs {
    /// Path to repository
    pub path: String,
    
    /// Path where the new worktree will be created
    pub worktree_path: String,
    
    /// Branch or commit to checkout in the worktree (optional, defaults to HEAD).
    /// Can be a branch name, tag, or commit SHA.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    
    /// Force creation even if worktree path already exists (default: false)
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_worktree_add` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitWorktreeAddPromptArgs {}

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
        false  // Creates worktree
    }
    
    fn destructive() -> bool {
        false  // Creates new files
    }
    
    fn idempotent() -> bool {
        false  // Fails if worktree exists
    }
    
    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);
        
        // Open repository
        let repo = crate::open_repo(path).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;
        
        // Build worktree add options
        let mut opts = crate::WorktreeAddOpts::new(&args.worktree_path);
        if let Some(ref branch) = args.branch {
            opts = opts.committish(branch);
        }
        opts = opts.force(args.force);
        
        // Execute worktree add
        let created_path = crate::worktree_add(repo, opts).await
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
