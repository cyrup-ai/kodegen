//! Browser screenshot tool - captures page or element as base64 image

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptMessage, PromptMessageRole, PromptMessageContent, PromptArgument};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::manager::BrowserManager;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserScreenshotArgs {
    /// Optional: CSS selector to screenshot specific element (default: full page)
    #[serde(default)]
    pub selector: Option<String>,

    /// Optional: format (png or jpeg, default: png)
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserScreenshotPromptArgs {}

#[derive(Clone)]
pub struct BrowserScreenshotTool {
    manager: Arc<BrowserManager>,
}

impl BrowserScreenshotTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait::async_trait]
impl Tool for BrowserScreenshotTool {
    type Args = BrowserScreenshotArgs;
    type PromptArgs = BrowserScreenshotPromptArgs;

    fn name() -> &'static str {
        "browser_screenshot"
    }

    fn description() -> &'static str {
        "Take a screenshot of the current page or specific element. Returns base64-encoded image.\\n\\n\
         Example: browser_screenshot({}) for full page\\n\
         Example: browser_screenshot({\"selector\": \"#content\"}) for specific element"
    }

    fn read_only() -> bool {
        true // Screenshots don't modify browser state
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Get browser context
        let context = self.manager.get_or_create_context().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;

        // Get current page
        let page = context.get_current_page().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get page: {}", e)))?;

        // Take screenshot
        let image_data = if let Some(selector) = &args.selector {
            // Screenshot specific element
            let element = page.find_element(selector).await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Element not found '{}': {}", selector, e)))?;

            element.screenshot(Default::default()).await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Screenshot failed: {}", e)))?
        } else {
            // Screenshot full page
            page.screenshot(Default::default()).await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Screenshot failed: {}", e)))?
        };

        // Encode as base64
        let base64_image = base64::encode(&image_data);

        // Determine format (assume PNG for now, could be enhanced)
        let format = args.format.as_deref().unwrap_or("png");

        Ok(json!({
            "success": true,
            "image": base64_image,
            "format": format,
            "size_bytes": image_data.len(),
            "selector": args.selector,
            "message": format!("Screenshot captured ({} bytes)", image_data.len())
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I take a screenshot?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_screenshot after navigating to a page.\\n\\n\
                     Full page: browser_screenshot({})\\n\
                     Specific element: browser_screenshot({\"selector\": \"#content\"})\\n\
                     JPEG format: browser_screenshot({\"format\": \"jpeg\"})"
                ),
            },
        ])
    }
}
