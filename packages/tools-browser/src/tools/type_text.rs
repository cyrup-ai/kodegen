//! Browser type text tool - inputs text into form fields

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptMessage, PromptMessageRole, PromptMessageContent, PromptArgument};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use crate::manager::BrowserManager;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserTypeTextArgs {
    /// CSS selector for input element
    pub selector: String,
    
    /// Text to type into the element
    pub text: String,
    
    /// Optional: clear existing text first (default: true)
    #[serde(default = "default_clear")]
    pub clear: bool,
    
    /// Optional: timeout in milliseconds (default: 5000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

fn default_clear() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserTypeTextPromptArgs {}

#[derive(Clone)]
pub struct BrowserTypeTextTool {
    manager: Arc<BrowserManager>,
}

impl BrowserTypeTextTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

impl Tool for BrowserTypeTextTool {
    type Args = BrowserTypeTextArgs;
    type PromptArgs = BrowserTypeTextPromptArgs;

    fn name() -> &'static str {
        "browser_type_text"
    }

    fn description() -> &'static str {
        "Type text into an input element using a CSS selector.\\n\\n\
         Automatically focuses element and clears existing text by default.\\n\\n\
         Example: browser_type_text({\\\"selector\\\": \\\"#email\\\", \\\"text\\\": \\\"user@example.com\\\"})\\n\
         Example: browser_type_text({\\\"selector\\\": \\\"#search\\\", \\\"text\\\": \\\"query\\\", \\\"clear\\\": false})"
    }

    fn read_only() -> bool {
        false // Typing changes page state
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate selector
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
        
        // Click element to focus
        element.click().await
            .map_err(|e| McpError::Other(anyhow::anyhow!(
                "Click to focus failed for '{}': {}", 
                args.selector,
                e
            )))?;
        
        // Clear existing text if requested
        if args.clear {
            // Select all (Ctrl+A) and delete
            element.press_key("Control").await.ok();  // Ignore errors (may not be needed)
            element.press_key("a").await.ok();
            element.press_key("Backspace").await.ok();
        }
        
        // Type text
        element.type_str(&args.text).await
            .map_err(|e| McpError::Other(anyhow::anyhow!(
                "Type text failed for '{}': {}", 
                args.selector,
                e
            )))?;
        
        Ok(json!({
            "success": true,
            "selector": args.selector,
            "text_length": args.text.len(),
            "cleared": args.clear,
            "message": format!(
                "Typed {} characters into: {}{}", 
                args.text.len(), 
                args.selector,
                if args.clear { " (cleared first)" } else { "" }
            )
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I type into a form field?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_type_text with selector and text. Examples:\\n\
                     - browser_type_text({\\\"selector\\\": \\\"#email\\\", \\\"text\\\": \\\"user@example.com\\\"})\\n\
                     - browser_type_text({\\\"selector\\\": \\\"input[name='password']\\\", \\\"text\\\": \\\"secret\\\"})\\n\
                     - browser_type_text({\\\"selector\\\": \\\"#search\\\", \\\"text\\\": \\\"query\\\", \\\"clear\\\": false})\\n\\n\
                     By default, existing text is cleared. Set clear: false to append."
                ),
            },
        ])
    }
}
