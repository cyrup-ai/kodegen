//! Browser screenshot tool - captures page or element as base64 image

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide_cdp::cdp::browser_protocol::page::CaptureScreenshotFormat;
use kodegen_mcp_schema::browser::{BrowserScreenshotArgs, BrowserScreenshotPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::manager::BrowserManager;

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

        // Normalize format string for display
        let format_str = match args.format.as_deref() {
            Some("jpeg") | Some("jpg") => "jpeg",
            Some("png") | None => "png",
            _ => "png",
        };

        // Create enum for chromiumoxide API
        let format_enum = match format_str {
            "jpeg" => CaptureScreenshotFormat::Jpeg,
            _ => CaptureScreenshotFormat::Png,
        };

        // Build screenshot params
        let screenshot_params = ScreenshotParams::builder()
            .format(format_enum.clone())
            .build();

        // Take screenshot (full page or element)
        let image_data = if let Some(selector) = &args.selector {
            // Element screenshot
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

            element.screenshot(format_enum.clone()).await.map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Element screenshot failed for selector '{}'. \
                     Possible causes: (1) Element is not visible or has no dimensions, \
                     (2) Element is obscured or off-screen, \
                     (3) Page is still loading. \
                     Error: {}",
                    selector,
                    e
                ))
            })?
        } else {
            // Full page screenshot
            page.screenshot(screenshot_params).await.map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Page screenshot failed. \
                     Possible causes: (1) Page has not fully loaded, \
                     (2) Page has excessive height or width, \
                     (3) Browser is in an invalid state. \
                     Error: {}",
                    e
                ))
            })?
        };

        // Encode as base64
        let base64_image = BASE64.encode(&image_data);

        Ok(json!({
            "success": true,
            "image": base64_image,
            "format": format_str,
            "size_bytes": image_data.len(),
            "selector": args.selector,
            "message": format!(
                "Screenshot captured ({} bytes, {} format{})",
                image_data.len(),
                format_str.to_uppercase(),
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
                     Note: Use after browser_navigate to ensure page is loaded.",
                ),
            },
        ])
    }
}
