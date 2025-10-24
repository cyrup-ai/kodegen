//! Browser screenshot tool - captures page or element as base64 image

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptMessage, PromptMessageRole, PromptMessageContent, PromptArgument};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide_cdp::cdp::browser_protocol::page::CaptureScreenshotFormat;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

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

impl Tool for BrowserScreenshotTool {
    type Args = BrowserScreenshotArgs;
    type PromptArgs = BrowserScreenshotPromptArgs;

    fn name() -> &'static str {
        "browser_screenshot"
    }

    fn description() -> &'static str {
        "Take a screenshot of the current page or specific element. Returns base64-encoded image.\\n\\n\
         Example: browser_screenshot({}) for full page\\n\
         Example: browser_screenshot({\\\"selector\\\": \\\"#content\\\"}) for specific element\\n\
         Example: browser_screenshot({\\\"format\\\": \\\"jpeg\\\"}) for smaller file size"
    }

    fn read_only() -> bool {
        true // Screenshots don't modify browser state
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Get browser instance
        let browser_arc = self.manager.get_or_launch().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;
        
        let browser_guard = browser_arc.lock().await;
        let wrapper = browser_guard.as_ref()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Browser not available")))?;
        
        // Create/get page
        let page = crate::browser::create_blank_page(wrapper).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get page: {}", e)))?;
        
        // Determine format
        let format = match args.format.as_deref() {
            Some("jpeg") | Some("jpg") => CaptureScreenshotFormat::Jpeg,
            _ => CaptureScreenshotFormat::Png,
        };
        
        // Build screenshot params
        let screenshot_params = ScreenshotParams::builder()
            .format(format.clone())
            .build();
        
        // Take screenshot (full page or element)
        let image_data = if let Some(selector) = &args.selector {
            // Element screenshot
            let element = page.find_element(selector).await
                .map_err(|e| McpError::Other(anyhow::anyhow!(
                    "Element not found '{}': {}", 
                    selector, 
                    e
                )))?;
            
            element.screenshot(format.clone()).await
                .map_err(|e| McpError::Other(anyhow::anyhow!(
                    "Element screenshot failed for '{}': {}", 
                    selector,
                    e
                )))?
        } else {
            // Full page screenshot
            page.screenshot(screenshot_params).await
                .map_err(|e| McpError::Other(anyhow::anyhow!(
                    "Page screenshot failed: {}",
                    e
                )))?
        };
        
        // Encode as base64
        let base64_image = BASE64.encode(&image_data);
        
        Ok(json!({
            "success": true,
            "image": base64_image,
            "format": if format == CaptureScreenshotFormat::Png { "png" } else { "jpeg" },
            "size_bytes": image_data.len(),
            "selector": args.selector,
            "message": format!(
                "Screenshot captured ({} bytes, {} format{})", 
                image_data.len(),
                if format == CaptureScreenshotFormat::Png { "PNG" } else { "JPEG" },
                if args.selector.is_some() { ", element only" } else { ", full page" }
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
                content: PromptMessageContent::text("How do I take a screenshot?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_screenshot after navigating to a page.\\n\\n\
                     Full page: browser_screenshot({})\\n\
                     Specific element: browser_screenshot({\\\"selector\\\": \\\"#content\\\"})\\n\
                     JPEG format (smaller): browser_screenshot({\\\"format\\\": \\\"jpeg\\\"})\\n\\n\
                     Note: Use after browser_navigate to ensure page is loaded."
                ),
            },
        ])
    }
}
