//! Git merge tool

use kodegen_mcp_tool::{Tool, error::McpError};
use kodegen_mcp_schema::git::{GitMergeArgs, GitMergePromptArgs};
use rmcp::model::{PromptArgument, PromptMessage};
use serde_json::{Value, json};
use std::path::Path;

/// Tool for merging branches
#[derive(Clone)]
pub struct GitMergeTool;

impl Tool for GitMergeTool {
    type Args = GitMergeArgs;
    type PromptArgs = GitMergePromptArgs;

    fn name() -> &'static str {
        "git_merge"
    }

    fn description() -> &'static str {
        "Merge a branch or commit into the current branch. \
         Joins two or more development histories together."
    }

    fn read_only() -> bool {
        false // Modifies HEAD and working tree
    }

    fn destructive() -> bool {
        true // Can overwrite local changes
    }

    fn idempotent() -> bool {
        false // Creates new commits
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        // Build merge options (note inverse logic for no_ff)
        let mut opts = crate::MergeOpts::new(&args.branch);
        opts = opts.no_ff(!args.fast_forward); // Inverse logic
        opts = opts.commit(args.auto_commit);

        // Execute merge
        let outcome = crate::merge(repo, opts)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let (merge_type, commit_id) = match outcome {
            crate::MergeOutcome::FastForward(id) => ("fast_forward", id.to_string()),
            crate::MergeOutcome::MergeCommit(id) => ("merge_commit", id.to_string()),
            crate::MergeOutcome::AlreadyUpToDate => {
                return Ok(json!({
                    "success": true,
                    "merge_type": "already_up_to_date",
                    "message": "Already up to date"
                }));
            }
        };

        Ok(json!({
            "success": true,
            "merge_type": merge_type,
            "commit_id": commit_id,
            "message": format!("Merged '{}' ({})", args.branch, merge_type)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
