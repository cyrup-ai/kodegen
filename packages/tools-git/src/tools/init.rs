//! Git repository initialization tool

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for initializing Git repositories
#[derive(Clone)]
pub struct GitInitTool;

/// Arguments for `git_init` tool
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GitInitArgs {
    /// Path where to initialize the repository
    pub path: String,

    /// Create a bare repository (no working directory)
    #[serde(default)]
    pub bare: bool,

    /// Name of the initial branch (informational only, gix uses default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_branch: Option<String>,
}

/// Prompt arguments for `git_init` tool
#[derive(Deserialize, JsonSchema)]
pub struct GitInitPromptArgs {}

impl Tool for GitInitTool {
    type Args = GitInitArgs;
    type PromptArgs = GitInitPromptArgs;

    fn name() -> &'static str {
        "git_init"
    }

    fn description() -> &'static str {
        "Initialize a new Git repository at the specified path. \
         Supports both normal repositories (with working directory) and \
         bare repositories (without working directory, typically for servers)."
    }

    fn read_only() -> bool {
        false // Creates files/directories
    }

    fn destructive() -> bool {
        false // Only creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Will fail if repo already exists
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Call appropriate function based on bare flag
        let task = if args.bare {
            crate::init_bare_repo(path)
        } else {
            crate::init_repo(path)
        };

        // Await AsyncTask, handle both layers of Result
        let _repo = task
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "path": args.path,
            "bare": args.bare,
            "message": format!("Initialized {} Git repository at {}",
                if args.bare { "bare" } else { "normal" }, args.path)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
