//! Browser navigation tool - loads URLs and waits for page ready

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptMessage, PromptMessageRole, PromptMessageContent, PromptArgument};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

use crate::manager::BrowserManager;

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
                "URL must start with http:// or https://"
            ));
        }
        
        // Get or create browser instance
        let browser_arc = self.manager.get_or_launch().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;
        
        let browser_guard = browser_arc.lock().await;
        let wrapper = browser_guard.as_ref()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Browser not available")))?;
        
        // Create new blank page (will close old page automatically)
        let page = wrapper.browser()
            .new_page("about:blank")
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create page: {}", e)))?;
        
        // Navigate to URL
        let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(30000));
        tokio::time::timeout(timeout, page.goto(&args.url))
            .await
            .map_err(|_| McpError::Other(anyhow::anyhow!(
                "Navigation timeout after {}ms for URL: {}", 
                timeout.as_millis(),
                args.url
            )))?
            .map_err(|e| McpError::Other(anyhow::anyhow!(
                "Navigation failed for {}: {}", 
                args.url,
                e
            )))?;
        
        // Wait for selector if specified (poll with find_element)
        if let Some(selector) = &args.wait_for_selector {
            let poll_interval = Duration::from_millis(100);
            let start = std::time::Instant::now();
            
            loop {
                if page.find_element(selector).await.is_ok() {
                    break;
                }
                
                if start.elapsed() >= timeout {
                    return Err(McpError::Other(anyhow::anyhow!(
                        "Selector '{}' not found after {}ms timeout", 
                        selector,
                        timeout.as_millis()
                    )));
                }
                
                tokio::time::sleep(poll_interval).await;
            }
        }
        
        // Get final URL (may differ from requested due to redirects)
        let final_url = page.url().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get URL: {}", e)))?
            .unwrap_or_else(|| args.url.clone());
        
        // CRITICAL: Store page for other tools to use (task 002)
        wrapper.set_current_page(page.clone()).await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to store page: {}", e)))?;
        
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
                     Increase timeout if needed: {\\\"url\\\": \\\"https://slow-site.com\\\", \\\"timeout_ms\\\": 60000}"
                ),
            },
        ])
    }
}
