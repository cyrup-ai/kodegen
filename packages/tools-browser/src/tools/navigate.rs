//! Browser navigation tool - loads URLs and waits for page ready

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

use crate::manager::BrowserManager;
use crate::utils::validate_navigation_timeout;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserNavigateArgs {
    /// URL to navigate to (must start with http:// or https://)
    pub url: String,

    /// Optional: wait for specific CSS selector before returning
    #[serde(default)]
    pub wait_for_selector: Option<String>,

    /// Optional: timeout in milliseconds (default: 30000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserNavigatePromptArgs {}

#[derive(Clone)]
pub struct BrowserNavigateTool {
    manager: Arc<BrowserManager>,
}

impl BrowserNavigateTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

impl Tool for BrowserNavigateTool {
    type Args = BrowserNavigateArgs;
    type PromptArgs = BrowserNavigatePromptArgs;

    fn name() -> &'static str {
        "browser_navigate"
    }

    fn description() -> &'static str {
        "Navigate to a URL in the browser. Opens the page and waits for load completion.\\n\\n\
         Returns current URL after navigation (may differ from requested URL due to redirects).\\n\\n\
         Example: browser_navigate({\\\"url\\\": \\\"https://example.com\\\"})\\n\
         With selector wait: browser_navigate({\\\"url\\\": \\\"https://example.com\\\", \\\"wait_for_selector\\\": \\\".content\\\"})"
    }

    fn read_only() -> bool {
        false // Navigation changes browser state
    }

    fn open_world() -> bool {
        true // Accesses external URLs
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate URL protocol
        if !args.url.starts_with("http://") && !args.url.starts_with("https://") {
            return Err(McpError::invalid_arguments(
                "URL must start with http:// or https://",
            ));
        }

        // Get or create browser instance
        let browser_arc = self
            .manager
            .get_or_launch()
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;

        let browser_guard = browser_arc.lock().await;
        let wrapper = browser_guard
            .as_ref()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!(
                "Browser not available. This is an internal error - please report it."
            )))?;

        // Close all existing pages to enforce single-page model
        // Prevents non-deterministic page selection in get_current_page()
        if let Ok(existing_pages) = wrapper.browser().pages().await {
            for page in existing_pages {
                // Ignore errors - pages might already be closed or unresponsive
                let _ = page.close().await;
            }
        }

        // Create new blank page (now guaranteed to be the ONLY page)
        let page = crate::browser::create_blank_page(wrapper)
            .await
            .map_err(|e| McpError::Other(e.into()))?;

        // Navigate to URL
        let timeout = validate_navigation_timeout(args.timeout_ms, 30000)?;
        tokio::time::timeout(timeout, page.goto(&args.url))
            .await
            .map_err(|_| {
                McpError::Other(anyhow::anyhow!(
                    "Navigation timeout after {}ms for URL: {}. \
                     Try: (1) Increase timeout_ms parameter (default: 30000), \
                     (2) Verify URL is accessible in a browser, \
                     (3) Check if site blocks headless browsers.",
                    timeout.as_millis(),
                    args.url
                ))
            })?
            .map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Navigation failed for URL: {}. \
                     Check: (1) URL is correctly formatted, \
                     (2) Network connectivity, \
                     (3) URL returns a valid HTTP response. \
                     Error: {}",
                    args.url, e
                ))
            })?;

        // Wait for selector if specified (exponential backoff)
        if let Some(selector) = &args.wait_for_selector {
            let start = std::time::Instant::now();
            let mut poll_interval = Duration::from_millis(100); // Start with 100ms
            let max_interval = Duration::from_secs(1);          // Cap at 1 second

            loop {
                // Try to find element
                if page.find_element(selector).await.is_ok() {
                    break;
                }

                // Check timeout
                if start.elapsed() >= timeout {
                    return Err(McpError::Other(anyhow::anyhow!(
                        "Selector '{}' not found after {}ms timeout. \
                         This selector was specified in wait_for_selector. \
                         Verify: (1) Selector exists on the target page, \
                         (2) Element loads within timeout period, \
                         (3) Consider increasing timeout_ms.",
                        selector,
                        timeout.as_millis()
                    )));
                }

                // Wait with exponential backoff
                tokio::time::sleep(poll_interval).await;
                
                // Double the interval, but cap at max_interval
                poll_interval = (poll_interval * 2).min(max_interval);
            }
        }

        // Get final URL (may differ from requested due to redirects)
        let final_url = page
            .url()
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get URL: {}", e)))?
            .unwrap_or_else(|| args.url.clone());

        Ok(json!({
            "success": true,
            "url": final_url,
            "requested_url": args.url,
            "redirected": final_url != args.url,
            "message": format!("Navigated to {}", final_url)
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I navigate to a website?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_navigate with a url parameter. Example: {\\\"url\\\": \\\"https://example.com\\\"}\\n\\n\
                     You can also wait for elements: {\\\"url\\\": \\\"https://example.com\\\", \\\"wait_for_selector\\\": \\\".content\\\"}\\n\
                     Increase timeout if needed: {\\\"url\\\": \\\"https://slow-site.com\\\", \\\"timeout_ms\\\": 60000}",
                ),
            },
        ])
    }
}
