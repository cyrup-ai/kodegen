//! Browser wait tool - pauses execution for specified duration

use kodegen_mcp_schema::browser::{BrowserWaitArgs, BrowserWaitPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};

use crate::utils::validate_wait_timeout;

#[derive(Clone, Default)]
pub struct BrowserWaitTool;

impl BrowserWaitTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for BrowserWaitTool {
    type Args = BrowserWaitArgs;
    type PromptArgs = BrowserWaitPromptArgs;

    fn name() -> &'static str {
        "browser_wait"
    }

    fn description() -> &'static str {
        "Wait for a specified duration (useful for waiting for dynamic content to load).\\n\\n\
         Example: browser_wait({\"duration_ms\": 2000}) - Wait 2 seconds"
    }

    fn read_only() -> bool {
        true // Waiting doesn't modify state
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate duration
        let duration = validate_wait_timeout(args.duration_ms)?;
        tokio::time::sleep(duration).await;

        Ok(json!({
            "success": true,
            "duration_ms": args.duration_ms,
            "message": format!("Waited {}ms", args.duration_ms)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I wait for content to load?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_wait to pause execution. Examples:\\n\
                     - browser_wait({\"duration_ms\": 1000}) - Wait 1 second\\n\
                     - browser_wait({\"duration_ms\": 5000}) - Wait 5 seconds\\n\\n\
                     For waiting for specific elements, use wait_for_selector in browser_navigate instead.",
                ),
            },
        ])
    }
}
