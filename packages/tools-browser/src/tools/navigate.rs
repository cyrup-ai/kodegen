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

#[async_trait::async_trait]
impl Tool for BrowserNavigateTool {
    type Args = BrowserNavigateArgs;
    type PromptArgs = BrowserNavigatePromptArgs;

    fn name() -> &'static str {
        "browser_navigate"
    }

    fn description() -> &'static str {
        "Navigate to a URL in the browser. Opens the page and waits for load completion.\\n\\n\
         Returns current URL after navigation.\\n\\n\
         Example: browser_navigate({\"url\": \"https://example.com\"})"
    }

    fn read_only() -> bool {
        false // Navigation changes browser state
    }

    fn open_world() -> bool {
        true // Accesses external URLs
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate URL
        if !args.url.starts_with("http://") && !args.url.starts_with("https://") {
            return Err(McpError::invalid_arguments(
                "URL must start with http:// or https://"
            ));
        }

        // Get or create browser context
        let context = self.manager.get_or_create_context().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;

        // Navigate with timeout
        let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(30000));

        tokio::time::timeout(timeout, context.navigate(&args.url))
            .await
            .map_err(|_| McpError::Other(anyhow::anyhow!("Navigation timeout after {}ms", timeout.as_millis())))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("Navigation failed: {}", e)))?;

        // Wait for selector if specified
        if let Some(selector) = &args.wait_for_selector {
            tokio::time::timeout(timeout, async {
                loop {
                    // Try to find the element
                    match context.get_current_page().await {
                        Ok(page) => {
                            if page.find_element(selector).await.is_ok() {
                                break;
                            }
                        }
                        Err(_) => continue,
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            })
                .await
                .map_err(|_| McpError::Other(anyhow::anyhow!("Selector wait timeout: {}", selector)))?;
        }

        // Get final URL
        let final_url = context.get_current_page().await
            .and_then(|page| page.url())
            .unwrap_or_else(|_| args.url.clone());

        Ok(json!({
            "success": true,
            "url": final_url,
            "requested_url": args.url,
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
                    "Use browser_navigate with a url parameter. Example: {\"url\": \"https://example.com\"}\\n\\n\
                     You can also wait for elements: {\"url\": \"https://example.com\", \"wait_for_selector\": \".content\"}"
                ),
            },
        ])
    }
}
