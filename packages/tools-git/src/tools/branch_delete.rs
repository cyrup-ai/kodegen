//! Git branch deletion tool

use kodegen_mcp_tool::{Tool, error::McpError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::{json, Value};
use rmcp::model::{PromptArgument, PromptMessage};
use std::path::Path;

/// Tool for deleting Git branches
#[derive(Clone)]
pub struct GitBranchDeleteTool;

/// Arguments for `git_branch_delete` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitBranchDeleteArgs {
    /// Path to repository
    pub path: String,
    
    /// Name of branch to delete
    pub branch: String,
    
    /// Force deletion
    #[serde(default)]
    pub force: bool,
}

/// Prompt arguments for `git_branch_delete` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitBranchDeletePromptArgs {}

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
        false  // Modifies repository
    }
    
    fn destructive() -> bool {
        true  // Deletes branches
    }
    
    fn idempotent() -> bool {
        false  // Will fail if branch doesn't exist
    }
    
    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);
        
        // Open repository
        let repo = crate::open_repo(path).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;
        
        // Delete branch
        crate::delete_branch(repo, args.branch.clone(), args.force).await
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
