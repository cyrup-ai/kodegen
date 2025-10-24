//! Browser click tool - clicks elements by CSS selector

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptMessage, PromptMessageRole, PromptMessageContent, PromptArgument};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use crate::manager::BrowserManager;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserClickArgs {
    /// CSS selector for element to click
    pub selector: String,
    
    /// Optional: timeout in milliseconds (default: 5000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserClickPromptArgs {}

#[derive(Clone)]
pub struct BrowserClickTool {
    manager: Arc<BrowserManager>,
}

impl BrowserClickTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

impl Tool for BrowserClickTool {
    type Args = BrowserClickArgs;
    type PromptArgs = BrowserClickPromptArgs;

    fn name() -> &'static str {
        "browser_click"
    }

    fn description() -> &'static str {
        "Click an element on the page using a CSS selector.\\n\\n\
         Automatically scrolls element into view before clicking.\\n\\n\
         Example: browser_click({\\\"selector\\\": \\\"#submit-button\\\"})\\n\
         Example: browser_click({\\\"selector\\\": \\\"button[type='submit']\\\"})"
    }

    fn read_only() -> bool {
        false // Clicking changes page state
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate selector not empty
        if args.selector.trim().is_empty() {
            return Err(McpError::invalid_arguments("Selector cannot be empty"));
        }
        
        // Get or create browser instance
        let browser_arc = self.manager.get_or_launch().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;
        
        let browser_guard = browser_arc.lock().await;
        let wrapper = browser_guard.as_ref()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Browser not available")))?;
        
        // Get current page (must call browser_navigate first)
        let page = crate::browser::get_current_page(wrapper).await
            .map_err(|e| McpError::Other(e))?;
        
        // Find element with timeout
        let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(5000));
        
        let element = tokio::time::timeout(timeout, page.find_element(&args.selector))
            .await
            .map_err(|_| McpError::Other(anyhow::anyhow!(
                "Element not found (timeout after {}ms): {}", 
                timeout.as_millis(),
                args.selector
            )))?
            .map_err(|e| McpError::Other(anyhow::anyhow!(
                "Element not found '{}': {}", 
                args.selector, 
                e
            )))?;
        
        // Click element (automatically scrolls into view)
        element.click().await
            .map_err(|e| McpError::Other(anyhow::anyhow!(
                "Click failed for '{}': {}", 
                args.selector,
                e
            )))?;
        
        Ok(json!({
            "success": true,
            "selector": args.selector,
            "message": format!("Clicked element: {}", args.selector)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I click a button?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_click with a CSS selector. Examples:\\n\
                     - browser_click({\\\"selector\\\": \\\"#submit\\\"}) - By ID\\n\
                     - browser_click({\\\"selector\\\": \\\".btn-primary\\\"}) - By class\\n\
                     - browser_click({\\\"selector\\\": \\\"button[type='submit']\\\"}) - By attribute\\n\
                     - browser_click({\\\"selector\\\": \\\"form button:first-child\\\"}) - Complex selector"
                ),
            },
        ])
    }
}
