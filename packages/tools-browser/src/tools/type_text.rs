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

#[async_trait::async_trait]
impl Tool for BrowserTypeTextTool {
    type Args = BrowserTypeTextArgs;
    type PromptArgs = BrowserTypeTextPromptArgs;

    fn name() -> &'static str {
        "browser_type_text"
    }

    fn description() -> &'static str {
        "Type text into an input element using a CSS selector.\\n\\n\
         Example: browser_type_text({\"selector\": \"#email\", \"text\": \"user@example.com\"})"
    }

    fn read_only() -> bool {
        false // Typing changes page state
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate selector and text
        if args.selector.trim().is_empty() {
            return Err(McpError::invalid_arguments("Selector cannot be empty"));
        }

        // Get browser context
        let context = self.manager.get_or_create_context().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;

        // Get current page
        let page = context.get_current_page().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get page: {}", e)))?;

        // Find element with timeout
        let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(5000));

        let element = tokio::time::timeout(timeout, page.find_element(&args.selector))
            .await
            .map_err(|_| McpError::Other(anyhow::anyhow!("Element not found (timeout): {}", args.selector)))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("Element not found '{}': {}", args.selector, e)))?;

        // Clear existing text if requested
        if args.clear {
            element.click().await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Click failed: {}", e)))?;

            // Select all and delete (Ctrl+A, Delete)
            element.press_key("Control").await.ok();
            element.press_key("a").await.ok();
            element.press_key("Backspace").await.ok();
        }

        // Type text
        element.type_str(&args.text).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Type failed: {}", e)))?;

        Ok(json!({
            "success": true,
            "selector": args.selector,
            "text_length": args.text.len(),
            "message": format!("Typed {} characters into: {}", args.text.len(), args.selector)
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
                     - browser_type_text({\"selector\": \"#email\", \"text\": \"user@example.com\"})\\n\
                     - browser_type_text({\"selector\": \"input[name='password']\", \"text\": \"secret\"})\\n\
                     - browser_type_text({\"selector\": \"#search\", \"text\": \"query\", \"clear\": false})"
                ),
            },
        ])
    }
}
