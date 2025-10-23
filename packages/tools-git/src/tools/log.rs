//! Git log tool

use kodegen_mcp_tool::{Tool, error::McpError};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::{json, Value};
use rmcp::model::{PromptArgument, PromptMessage};
use std::path::Path;
use tokio_stream::StreamExt;

/// Tool for listing Git commit history
#[derive(Clone)]
pub struct GitLogTool;

/// Arguments for `git_log` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitLogArgs {
    /// Path to repository
    pub path: String,
    
    /// Maximum number of commits to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_count: Option<usize>,
    
    /// Number of commits to skip
    #[serde(default)]
    pub skip: usize,
    
    /// Filter commits by file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_filter: Option<String>,
}

/// Prompt arguments for `git_log` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitLogPromptArgs {}

impl Tool for GitLogTool {
    type Args = GitLogArgs;
    type PromptArgs = GitLogPromptArgs;
    
    fn name() -> &'static str {
        "git_log"
    }
    
    fn description() -> &'static str {
        "List commit history from a Git repository. \
         Optionally filter by file path and limit the number of results."
    }
    
    fn read_only() -> bool {
        true  // Only reads, doesn't modify
    }
    
    fn destructive() -> bool {
        false
    }
    
    fn idempotent() -> bool {
        true  // Safe to call repeatedly
    }
    
    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);
        
        // Open repository
        let repo = crate::open_repo(path).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;
        
        // Build log options
        let mut opts = crate::LogOpts::new();
        
        if let Some(max_count) = args.max_count {
            opts = opts.max_count(max_count + args.skip);
        }
        
        if let Some(path_filter) = args.path_filter {
            opts = opts.path(path_filter);
        }
        
        // Get log stream
        let mut stream = crate::log(repo, opts);
        
        // Collect commits
        let mut commits = Vec::new();
        let mut skipped = 0;
        
        while let Some(result) = stream.next().await {
            match result {
                Ok(commit_info) => {
                    // Skip first N commits if requested
                    if skipped < args.skip {
                        skipped += 1;
                        continue;
                    }
                    
                    commits.push(json!({
                        "id": commit_info.id.to_string(),
                        "author": {
                            "name": commit_info.author.name,
                            "email": commit_info.author.email,
                            "time": commit_info.author.time.to_rfc3339()
                        },
                        "summary": commit_info.summary,
                        "time": commit_info.time.to_rfc3339()
                    }));
                }
                Err(e) => {
                    return Err(McpError::Other(anyhow::anyhow!("{e}")));
                }
            }
        }
        
        Ok(json!({
            "success": true,
            "commits": commits,
            "count": commits.len()
        }))
    }
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
    
    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
