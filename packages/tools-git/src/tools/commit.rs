//! Git commit tool

use kodegen_mcp_tool::{Tool, error::McpError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::{json, Value};
use rmcp::model::{PromptArgument, PromptMessage};
use std::path::Path;

/// Tool for creating Git commits
#[derive(Clone)]
pub struct GitCommitTool;

/// Arguments for `git_commit` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitCommitArgs {
    /// Path to repository
    pub path: String,
    
    /// Commit message
    pub message: String,
    
    /// Author name (optional, uses git config if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    
    /// Author email (optional, uses git config if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_email: Option<String>,
    
    /// Stage all modified tracked files before committing
    #[serde(default)]
    pub all: bool,
}

/// Prompt arguments for `git_commit` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitCommitPromptArgs {}

impl Tool for GitCommitTool {
    type Args = GitCommitArgs;
    type PromptArgs = GitCommitPromptArgs;
    
    fn name() -> &'static str {
        "git_commit"
    }
    
    fn description() -> &'static str {
        "Create a new commit in a Git repository. \
         Optionally specify author information and stage all modified files."
    }
    
    fn read_only() -> bool {
        false  // Creates commits
    }
    
    fn destructive() -> bool {
        false  // Only creates, doesn't delete
    }
    
    fn idempotent() -> bool {
        false  // Creates new commits each time
    }
    
    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);
        
        // Open repository
        let repo = crate::open_repo(path).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;
        
        // Build commit options
        let mut opts = crate::CommitOpts::message(&args.message);
        opts = opts.all(args.all);
        
        // Set author if provided
        if let (Some(name), Some(email)) = (args.author_name, args.author_email) {
            let author = crate::Signature::new(name, email);
            opts = opts.author(author);
        }
        
        // Create commit
        let commit_id = crate::commit(repo, opts).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;
        
        Ok(json!({
            "success": true,
            "commit_id": commit_id.to_string(),
            "message": args.message
        }))
    }
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
    
    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
