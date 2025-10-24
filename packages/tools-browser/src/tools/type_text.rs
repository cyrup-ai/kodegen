//! Browser type text tool - inputs text into form fields

use kodegen_mcp_schema::browser::{BrowserTypeTextArgs, BrowserTypeTextPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::manager::BrowserManager;
use crate::utils::validate_interaction_timeout;

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
        let browser_arc = self
            .manager
            .get_or_launch()
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;

        let browser_guard = browser_arc.lock().await;
        let wrapper = browser_guard.as_ref().ok_or_else(|| {
            McpError::Other(anyhow::anyhow!(
                "Browser not available. This is an internal error - please report it."
            ))
        })?;

        // Get current page (must call browser_navigate first)
        let page = crate::browser::get_current_page(wrapper)
            .await
            .map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Failed to get page. Did you call browser_navigate first? Error: {}",
                    e
                ))
            })?;

        // Find element with timeout
        let timeout = validate_interaction_timeout(args.timeout_ms, 5000)?;

        let element = tokio::time::timeout(timeout, page.find_element(&args.selector))
            .await
            .map_err(|_| {
                McpError::Other(anyhow::anyhow!(
                    "Element not found (timeout after {}ms): '{}'. \
                     Try: (1) Verify selector is correct using browser dev tools, \
                     (2) Use browser_wait_for to wait for element to appear, \
                     (3) Increase timeout_ms parameter.",
                    timeout.as_millis(),
                    args.selector
                ))
            })?
            .map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Element not found for selector '{}'. \
                     Verify: (1) Selector syntax is valid CSS, \
                     (2) Element exists on current page, \
                     (3) Element is not in an iframe (unsupported). \
                     Error: {}",
                    args.selector,
                    e
                ))
            })?;

        // Click element to focus
        element.click().await.map_err(|e| {
            McpError::Other(anyhow::anyhow!(
                "Click to focus failed for selector '{}'. \
                 Possible causes: (1) Element is obscured by another element, \
                 (2) Element is disabled or not focusable, \
                 (3) Page is still loading. \
                 Error: {}",
                args.selector,
                e
            ))
        })?;

        // Clear existing text if requested
        if args.clear {
            element
                .call_js_fn("function() { this.value = ''; }", false)
                .await
                .map_err(|e| {
                    McpError::Other(anyhow::anyhow!(
                        "Failed to clear field for selector '{}'. \
                         Possible causes: (1) Element is not an input/textarea field, \
                         (2) Field is read-only or disabled, \
                         (3) JavaScript execution was blocked. \
                         Error: {}",
                        args.selector,
                        e
                    ))
                })?;
        }

        // Type text
        element.type_str(&args.text).await.map_err(|e| {
            McpError::Other(anyhow::anyhow!(
                "Type text failed for selector '{}'. \
                 Possible causes: (1) Element lost focus during typing, \
                 (2) Element is not a text input field, \
                 (3) Field has input restrictions or validation. \
                 Error: {}",
                args.selector,
                e
            ))
        })?;

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
                     By default, existing text is cleared. Set clear: false to append.",
                ),
            },
        ])
    }
}
