//! Git fetch tool

use kodegen_tool::{Tool, error::McpError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::{json, Value};
use rmcp::model::{PromptArgument, PromptMessage};
use std::path::Path;

/// Tool for fetching from remote repositories
#[derive(Clone)]
pub struct GitFetchTool;

fn default_remote() -> String {
    "origin".to_string()
}

/// Arguments for git_fetch tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitFetchArgs {
    /// Path to repository
    pub path: String,
    
    /// Remote name (defaults to "origin")
    #[serde(default = "default_remote")]
    pub remote: String,
    
    /// Refspecs to fetch (e.g., ["refs/heads/main:refs/remotes/origin/main"]).
    /// If empty, uses repository's configured refspecs for the remote.
    #[serde(default)]
    pub refspecs: Vec<String>,
    
    /// Prune remote-tracking branches that no longer exist on remote (default: false)
    #[serde(default)]
    pub prune: bool,
}

/// Prompt arguments for git_fetch tool
#[derive(Deserialize, JsonSchema)]
pub struct GitFetchPromptArgs {}

impl Tool for GitFetchTool {
    type Args = GitFetchArgs;
    type PromptArgs = GitFetchPromptArgs;
    
    fn name() -> &'static str {
        "git_fetch"
    }
    
    fn description() -> &'static str {
        "Fetch updates from a remote repository. \
         Downloads objects and refs from another repository."
    }
    
    fn read_only() -> bool {
        false  // Fetches refs
    }
    
    fn destructive() -> bool {
        false  // Only adds, doesn't delete except with prune
    }
    
    fn idempotent() -> bool {
        true  // Safe to fetch repeatedly
    }
    
    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);
        
        // Open repository
        let repo = crate::open_repo(path).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {}", e)))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;
        
        // Build fetch options
        let mut opts = crate::FetchOpts::from_remote(&args.remote);
        for refspec in &args.refspecs {
            opts = opts.add_refspec(refspec);
        }
        opts = opts.prune(args.prune);
        
        // Execute fetch
        crate::fetch(repo, opts).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {}", e)))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{}", e)))?;
        
        Ok(json!({
            "success": true,
            "remote": args.remote,
            "pruned": args.prune
        }))
    }
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
    
    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
