//! Git repository opening tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitOpenArgs, GitOpenPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for opening existing Git repositories
#[derive(Clone)]
pub struct GitOpenTool;

impl Tool for GitOpenTool {
    type Args = GitOpenArgs;
    type PromptArgs = GitOpenPromptArgs;

    fn name() -> &'static str {
        "git_open"
    }

    fn description() -> &'static str {
        "Open an existing Git repository at the specified path. \
         The repository must already exist at the given location."
    }

    fn read_only() -> bool {
        true // Only reads, doesn't modify
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true // Opening same repo multiple times is safe
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        let _repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "path": args.path,
            "message": format!("Opened Git repository at {}", args.path)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
