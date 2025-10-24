//! Browser wait_for tool - waits for elements to meet specific conditions

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

use crate::manager::BrowserManager;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WaitCondition {
    /// Element exists in DOM (uses page.find_element)
    Present,

    /// Element is visible (not display:none, visibility:hidden, etc)
    Visible,

    /// Element is visible AND not disabled (ready for interaction)
    Clickable,

    /// Element text contains specific string
    TextContains,

    /// Element attribute has specific value
    AttributeIs,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserWaitForArgs {
    /// CSS selector for element to wait for
    pub selector: String,

    /// Condition to wait for
    pub condition: WaitCondition,

    /// Optional: timeout in milliseconds (default: 10000)
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Optional: expected text (required for TextContains condition)
    #[serde(default)]
    pub text: Option<String>,

    /// Optional: attribute name (required for AttributeIs condition)
    #[serde(default)]
    pub attribute_name: Option<String>,

    /// Optional: attribute value (required for AttributeIs condition)
    #[serde(default)]
    pub attribute_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserWaitForPromptArgs {}

#[derive(Clone)]
pub struct BrowserWaitForTool {
    manager: Arc<BrowserManager>,
}

impl BrowserWaitForTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

impl Tool for BrowserWaitForTool {
    type Args = BrowserWaitForArgs;
    type PromptArgs = BrowserWaitForPromptArgs;

    fn name() -> &'static str {
        "browser_wait_for"
    }

    fn description() -> &'static str {
        "Wait for an element to meet a specific condition before proceeding.\\n\\n\
         Supports multiple wait conditions:\\n\
         - present: Element exists in DOM\\n\
         - visible: Element is visible (not display:none, visibility:hidden, opacity:0)\\n\
         - clickable: Element is visible and not disabled\\n\
         - text_contains: Element text contains specific string\\n\
         - attribute_is: Element attribute has specific value\\n\\n\
         Example: browser_wait_for({\\\"selector\\\": \\\"#results\\\", \\\"condition\\\": \\\"visible\\\", \\\"timeout_ms\\\": 10000})"
    }

    fn read_only() -> bool {
        false // Waiting changes perceived state
    }

    fn open_world() -> bool {
        false // Operates on current page only
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate selector not empty
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
        let wrapper = browser_guard
            .as_ref()
            .ok_or_else(|| McpError::Other(anyhow::anyhow!("Browser not available")))?;

        // Get current page (must call browser_navigate first)
        let page = crate::browser::get_current_page(wrapper)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get page: {}", e)))?;

        // Setup timeout and polling
        let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(10000));
        let start = std::time::Instant::now();
        let mut poll_interval = Duration::from_millis(100); // Start with 100ms
        let max_interval = Duration::from_secs(2);          // Cap at 2 seconds

        loop {
            // Try to find element
            match page.find_element(&args.selector).await {
                Ok(element) => {
                    // Element found, now check condition
                    match check_condition(&element, &args).await {
                        Ok(true) => {
                            // Condition met!
                            return Ok(json!({
                                "success": true,
                                "selector": args.selector,
                                "condition": format!("{:?}", args.condition),
                                "elapsed_ms": start.elapsed().as_millis() as u64,
                                "message": format!("Condition {:?} met for selector: {}", args.condition, args.selector)
                            }));
                        }
                        Ok(false) => {
                            // Condition not met yet, continue polling
                        }
                        Err(e) => return Err(e), // Unrecoverable error
                    }
                }
                Err(_) if matches!(args.condition, WaitCondition::Present) => {
                    // For "present" condition, not finding element means condition not met
                    // Continue polling
                }
                Err(_) if start.elapsed() >= timeout => {
                    // Timeout reached and element not found
                    return Err(McpError::Other(anyhow::anyhow!(
                        "Element '{}' not found after {}ms timeout",
                        args.selector, timeout.as_millis()
                    )));
                }
                Err(_) => {
                    // Element not found yet, but we have time left
                    // Continue polling
                }
            }
