//! Git repository discovery tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitDiscoverArgs, GitDiscoverPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for discovering Git repositories by searching upward
#[derive(Clone)]
pub struct GitDiscoverTool;

impl Tool for GitDiscoverTool {
    type Args = GitDiscoverArgs;
    type PromptArgs = GitDiscoverPromptArgs;

    fn name() -> &'static str {
        "git_discover"
    }

    fn description() -> &'static str {
        "Discover a Git repository by searching upward from the given path. \
         This will traverse parent directories until it finds a .git directory \
         or reaches the filesystem root."
    }

    fn read_only() -> bool {
        true // Only searches, doesn't modify
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Safe to call repeatedly
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        let _repo = crate::discover_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "searched_from": args.path,
            "message": format!("Discovered Git repository from path {}", args.path)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
