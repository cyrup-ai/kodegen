//! Browser scroll tool - scrolls page or to specific element

use chromiumoxide_cdp::cdp::js_protocol::runtime::{CallArgument, CallFunctionOnParams};
use kodegen_mcp_schema::browser::{BrowserScrollArgs, BrowserScrollPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::warn;

use crate::manager::BrowserManager;

#[derive(Clone)]
pub struct BrowserScrollTool {
    manager: Arc<BrowserManager>,
}

impl BrowserScrollTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

impl Tool for BrowserScrollTool {
    type Args = BrowserScrollArgs;
    type PromptArgs = BrowserScrollPromptArgs;

    fn name() -> &'static str {
        "browser_scroll"
    }

    fn description() -> &'static str {
        "Scroll the page by amount or to a specific element.\\n\\n\
         Examples:\\n\
         - browser_scroll({\"y\": 500}) - Scroll down 500px\\n\
         - browser_scroll({\"selector\": \"#footer\"}) - Scroll to element"
    }

    fn read_only() -> bool {
        false // Scrolling changes viewport state
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Get browser instance
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

        // Perform scroll
        if let Some(selector) = &args.selector {
            // Find element first (validates existence)
            let element = page.find_element(selector).await.map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Element not found for selector '{}'. \
                     Verify: (1) Selector syntax is valid CSS, \
                     (2) Element exists on current page, \
                     (3) Element is not in an iframe (unsupported). \
                     Error: {}",
                    selector,
                    e
                ))
            })?;

            // Use chromiumoxide's scroll_into_view() (has IntersectionObserver check)
            element.scroll_into_view().await.map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Scroll to element failed. \
                     Possible causes: (1) Element is not scrollable or not in viewport, \
                     (2) Page structure prevents scrolling, \
                     (3) Element is detached from DOM. \
                     Error: {}",
                    e
                ))
            })?;

            Ok(json!({
                "success": true,
                "action": "scroll_to_element",
                "selector": selector,
                "message": format!("Scrolled to element: {}", selector)
            }))
        } else {
            // Scroll by amount
            // Validate scroll amounts (defense-in-depth)
            // Agent validates, but tool should also validate since it's a public MCP tool
            let x = args.x.unwrap_or(0).clamp(-10_000, 10_000);
            let y = args.y.unwrap_or(0).clamp(-10_000, 10_000);

            // Warn if attempting to scroll zero pixels
            if x == 0 && y == 0 {
                warn!("Scroll called with x=0, y=0 (no-op)");
            }

            // Safe: parameterized evaluation prevents injection
            let call = CallFunctionOnParams::builder()
                .function_declaration("(x, y) => window.scrollBy(x, y)")
                .argument(CallArgument::builder().value(json!(x)).build())
                .argument(CallArgument::builder().value(json!(y)).build())
                .build()
                .map_err(|e| {
                    McpError::Other(anyhow::anyhow!("Failed to build scroll params: {}", e))
                })?;

            page.evaluate_function(call).await.map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Scroll by amount failed. \
                     Possible causes: (1) Page does not support scrolling, \
                     (2) Scroll amount exceeds page boundaries, \
                     (3) JavaScript execution was blocked. \
                     Error: {}",
                    e
                ))
            })?;

            Ok(json!({
                "success": true,
                "action": "scroll_by_amount",
                "x": x,
                "y": y,
                "message": format!("Scrolled by x={}, y={}", x, y)
            }))
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I scroll a page?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_scroll to scroll the page. Examples:\\n\
                     - browser_scroll({\"y\": 500}) - Scroll down 500px\\n\
                     - browser_scroll({\"y\": -300}) - Scroll up 300px\\n\
                     - browser_scroll({\"x\": 200, \"y\": 400}) - Scroll right and down\\n\
                     - browser_scroll({\"selector\": \"#footer\"}) - Scroll to element",
                ),
            },
        ])
    }
}
