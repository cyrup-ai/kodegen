//! Git repository cloning tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitCloneArgs, GitClonePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};

/// Tool for cloning remote Git repositories
#[derive(Clone)]
pub struct GitCloneTool;

impl Tool for GitCloneTool {
    type Args = GitCloneArgs;
    type PromptArgs = GitClonePromptArgs;

    fn name() -> &'static str {
        "git_clone"
    }

    fn description() -> &'static str {
        "Clone a remote Git repository to a local path. \
         Supports shallow cloning (limited history) and branch-specific cloning. \
         The destination path must not already exist."
    }

    fn read_only() -> bool {
        false // Creates files/directories
    }

    fn destructive() -> bool {
        false // Only creates, doesn't delete
    }

    fn idempotent() -> bool {
        false // Will fail if destination exists
    }

    fn open_world() -> bool {
        true // Makes network requests
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let mut opts = crate::CloneOpts::new(&args.url, &args.path);

        if let Some(depth) = args.depth {
            opts = opts.shallow(depth);
        }

        if let Some(ref branch) = args.branch {
            opts = opts.branch(branch);
        }

        let _repo = crate::clone_repo(opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        Ok(json!({
            "success": true,
            "url": args.url,
            "path": args.path,
            "branch": args.branch,
            "shallow": args.depth.is_some(),
            "message": format!("Cloned {} to {}", args.url, args.path)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
