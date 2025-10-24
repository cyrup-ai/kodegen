//! Browser scroll tool - scrolls page or to specific element

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use chromiumoxide_cdp::cdp::js_protocol::runtime::{CallFunctionOnParams, CallArgument};

use crate::manager::BrowserManager;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserScrollArgs {
    /// Optional: CSS selector to scroll to element (takes priority over x/y)
    #[serde(default)]
    pub selector: Option<String>,

    /// Optional: horizontal scroll amount in pixels (default: 0)
    #[serde(default)]
    pub x: Option<i32>,

    /// Optional: vertical scroll amount in pixels (default: 0)
    #[serde(default)]
    pub y: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserScrollPromptArgs {}

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
        let wrapper = browser_guard
            .as_ref()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Browser not available")))?;

        // Get current page (must call browser_navigate first)
        let page = crate::browser::get_current_page(wrapper)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get page: {}", e)))?;

        // Perform scroll
        if let Some(selector) = &args.selector {
            // Find element first (validates existence)
            let element = page.find_element(selector).await.map_err(|e| {
                McpError::Other(anyhow::anyhow!("Element not found '{}': {}", selector, e))
            })?;

            // Use chromiumoxide's scroll_into_view() (has IntersectionObserver check)
            element
                .scroll_into_view()
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Scroll failed: {}", e)))?;

            Ok(json!({
                "success": true,
                "action": "scroll_to_element",
                "selector": selector,
                "message": format!("Scrolled to element: {}", selector)
            }))
        } else {
            // Scroll by amount
            let x = args.x.unwrap_or(0);
            let y = args.y.unwrap_or(0);

            // Safe: parameterized evaluation prevents injection
            let call = CallFunctionOnParams::builder()
                .function_declaration("(x, y) => window.scrollBy(x, y)")
                .argument(CallArgument::builder().value(json!(x)).build())
                .argument(CallArgument::builder().value(json!(y)).build())
                .build()
                .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to build scroll params: {}", e)))?;

            page.evaluate_function(call)
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Scroll failed: {}", e)))?;

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
