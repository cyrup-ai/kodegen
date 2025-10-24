//! Git checkout tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitCheckoutArgs, GitCheckoutPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for checking out Git references
#[derive(Clone)]
pub struct GitCheckoutTool;

impl Tool for GitCheckoutTool {
    type Args = GitCheckoutArgs;
    type PromptArgs = GitCheckoutPromptArgs;

    fn name() -> &'static str {
        "git_checkout"
    }

    fn description() -> &'static str {
        "Checkout a Git reference (branch, tag, or commit) or restore specific files. \
         Without paths: switches branches/commits. With paths: restores files from the reference."
    }

    fn read_only() -> bool {
        false // Modifies working directory
    }

    fn destructive() -> bool {
        true // Can discard local changes with force
    }

    fn idempotent() -> bool {
        true // Checking out same ref multiple times is safe
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // If create flag is set, create the branch first
        if args.create {
            let branch_opts = crate::BranchOpts {
                name: args.target.clone(),
                start_point: None, // Use HEAD
                force: false,
                checkout: false, // We'll checkout separately
                track: false,
            };

            crate::branch(repo.clone(), branch_opts)
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
                .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;
        }

        // Build checkout options
        let mut opts = crate::CheckoutOpts::new(&args.target);
        opts = opts.force(args.force);

        // Add file paths if specified
        if let Some(ref file_paths) = args.paths {
            opts = opts.paths(file_paths.iter().map(std::string::String::as_str));
        }

        // Execute checkout
        crate::checkout(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let message = if args.create {
            format!("Created and checked out branch '{}'", args.target)
        } else if let Some(ref paths) = args.paths {
            format!("Restored {} file(s) from '{}'", paths.len(), args.target)
        } else {
            format!("Checked out '{}'", args.target)
        };

        Ok(json!({
            "success": true,
            "target": args.target,
            "created": args.create,
            "paths": args.paths,
            "message": message
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
