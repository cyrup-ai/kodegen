//! GitHub authenticated user retrieval tool

use anyhow;
use kodegen_mcp_schema::github::{GetMeArgs, GetMePromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use serde_json::Value;

/// Tool for getting information about the authenticated GitHub user
#[derive(Clone)]
pub struct GetMeTool;

impl Tool for GetMeTool {
    type Args = GetMeArgs;
    type PromptArgs = GetMePromptArgs;
    
    fn name() -> &'static str {
        "get_me"
    }
    
    fn description() -> &'static str {
        "Get information about the authenticated GitHub user. Returns user profile \
         details including login, name, email, avatar, bio, company, location, repos, \
         followers, etc. Uses GITHUB_TOKEN for authentication. No arguments needed."
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
    
    async fn execute(&self, _args: Self::Args) -> Result<Value, McpError> {
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
        
        // Call API wrapper (returns AsyncTask<Result<Author, GitHubError>>)
        let task_result = client.get_me().await;
        
        // Handle outer Result (channel error)
        let api_result = task_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task channel error: {}", e)))?;
        
        // Handle inner Result (GitHub API error)
        let user = api_result
            .map_err(|e| McpError::Other(anyhow::anyhow!("GitHub API error: {}", e)))?;
        
        // Return serialized user
        Ok(serde_json::to_value(&user)?)
    }
    
    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
    
    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I get information about the authenticated user?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use get_me to retrieve your GitHub user details:\n\n\
                     get_me({})\n\n\
                     Returns information about the user associated with the GITHUB_TOKEN:\n\
                     - login: Username\n\
                     - id: User ID\n\
                     - name: Display name\n\
                     - email: Email address\n\
                     - avatar_url: Profile image\n\
                     - bio: User bio\n\
                     - company: Company name\n\
                     - location: Location\n\
                     - blog: Website URL\n\
                     - public_repos: Number of public repositories\n\
                     - followers: Follower count\n\
                     - following: Following count\n\
                     - created_at: Account creation date\n\n\
                     Use this to:\n\
                     - Verify authentication is working\n\
                     - Get your username for other operations\n\
                     - Display user information\n\
                     - Check account details\n\n\
                     No arguments needed - automatically uses GITHUB_TOKEN.\n\n\
                     Requirements:\n\
                     - GITHUB_TOKEN environment variable must be set\n\
                     - Token must be valid and not expired"
                ),
            },
        ])
    }
}
