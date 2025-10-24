//! GitHub code scanning alerts tool

use anyhow;
use kodegen_mcp_schema::github::{CodeScanningAlertsArgs, CodeScanningAlertsPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use serde_json::Value;

/// Tool for listing code scanning security alerts in a GitHub repository
#[derive(Clone)]
pub struct CodeScanningAlertsTool;

impl Tool for CodeScanningAlertsTool {
    type Args = CodeScanningAlertsArgs;
    type PromptArgs = CodeScanningAlertsPromptArgs;
    
    fn name() -> &'static str {
        "code_scanning_alerts"
    }
    
    fn description() -> &'static str {
        "List code scanning security alerts for a GitHub repository. Returns alerts \
         with details about vulnerabilities, their severity, location, and status. \
         Supports filtering by state, branch, tool, and severity. Requires GitHub \
         Advanced Security enabled. Requires GITHUB_TOKEN environment variable."
    }
    
    fn read_only() -> bool {
        true  // Only reads data
    }
    
    fn destructive() -> bool {
        false  // No destructive operations
    }
    
    fn idempotent() -> bool {
        true  // Same request returns same result
    }
    
    fn open_world() -> bool {
        true  // Calls external GitHub API
    }
    
    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Get GitHub token from environment
        let token = std::env::var("GITHUB_TOKEN")
            .map_err(|_| McpError::Other(anyhow::anyhow!(
                "GITHUB_TOKEN environment variable not set"
            )))?;
        
        // Build GitHub client
        let client = crate::GitHubClient::builder()
            .personal_token(token)
            .build()
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create GitHub client: {}", e)))?;
        
        // Call API wrapper (returns AsyncTask<Result<Vec<Value>, GitHubError>>)
        let task_result = client.list_code_scanning_alerts(
            args.owner,
            args.repo,
            args.state,
            args.ref_name,
            args.tool_name,
            args.severity,
        ).await;
        
        // Handle outer Result (channel error)
        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;
        
        // Handle inner Result (GitHub API error)
        let alerts = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;
        
        // Return serialized alerts (Vec<serde_json::Value>)
        Ok(serde_json::to_value(&alerts)?)
    }
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
    
    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I get code scanning alerts for a repository?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use code_scanning_alerts to retrieve security alerts:\n\n\
                     # Get all open alerts\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\"\n\
                     })\n\n\
                     # Get critical severity alerts\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"state\": \"open\",\n\
                       \"severity\": \"critical\"\n\
                     })\n\n\
                     # Get alerts for specific branch\n\
                     code_scanning_alerts({\n\
                       \"owner\": \"octocat\",\n\
                       \"repo\": \"hello-world\",\n\
                       \"ref_name\": \"main\",\n\
                       \"state\": \"open\"\n\
                     })\n\n\
                     States: \"open\", \"closed\", \"dismissed\"\n\
                     Severities: \"critical\", \"high\", \"medium\", \"low\", \"warning\", \"note\", \"error\"\n\n\
                     Each alert includes:\n\
                     - Alert number and state\n\
                     - Severity and description\n\
                     - Location (file, line)\n\
                     - Tool that found it (CodeQL, etc.)\n\
                     - Created/updated timestamps\n\
                     - Dismissal reason (if dismissed)\n\n\
                     Requires: GitHub Advanced Security enabled on the repository.\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token needs 'security_events' scope\n\
                     - Repository must have code scanning enabled\n\
                     - User must have appropriate permissions"
                ),
            },
        ])
    }
}
