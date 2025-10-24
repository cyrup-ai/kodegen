//! Browser wait_for tool - waits for elements to meet specific conditions

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

use crate::manager::BrowserManager;
use chromiumoxide_cdp::cdp::js_protocol::runtime::{CallFunctionOnParams, CallArgument};

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

            // Check timeout
            if start.elapsed() >= timeout {
                return Err(McpError::Other(anyhow::anyhow!(
                    "Condition '{:?}' not met for selector '{}' after {}ms timeout",
                    args.condition, args.selector, timeout.as_millis()
                )));
            }

            // Wait with exponential backoff
            tokio::time::sleep(poll_interval).await;
            poll_interval = (poll_interval * 2).min(max_interval);
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I wait for dynamic content to load?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_wait_for to wait for elements to meet specific conditions:\\n\\n\
                     Wait for element to appear:\\n\
                     browser_wait_for({\\\"selector\\\": \\\"#results\\\", \\\"condition\\\": \\\"present\\\"})\\n\\n\
                     Wait for element to be visible:\\n\
                     browser_wait_for({\\\"selector\\\": \\\".content\\\", \\\"condition\\\": \\\"visible\\\"})\\n\\n\
                     Wait for button to be clickable:\\n\
                     browser_wait_for({\\\"selector\\\": \\\"#submit\\\", \\\"condition\\\": \\\"clickable\\\"})\\n\\n\
                     Wait for text to appear:\\n\
                     browser_wait_for({\\\"selector\\\": \\\"#status\\\", \\\"condition\\\": \\\"text_contains\\\", \\\"text\\\": \\\"Complete\\\"})\\n\\n\
                     Wait for attribute value:\\n\
                     browser_wait_for({\\\"selector\\\": \\\"input\\\", \\\"condition\\\": \\\"attribute_is\\\", \\\"attribute_name\\\": \\\"aria-invalid\\\", \\\"attribute_value\\\": \\\"false\\\"})",
                ),
            },
        ])
    }
}

/// Check if element meets the specified condition
async fn check_condition(
    element: &chromiumoxide::element::Element,
    args: &BrowserWaitForArgs,
) -> Result<bool, McpError> {
    match args.condition {
        WaitCondition::Present => {
            // If we got here, element exists
            Ok(true)
        }

        WaitCondition::Visible => {
            // Use JavaScript to check computed style and bounding box
            let js_fn = r#"
                function() {
                    const style = window.getComputedStyle(this);
                    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') {
                        return false;
                    }
                    const rect = this.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0;
                }
            "#;
            
            let result = element.call_js_fn(js_fn, false).await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to check visibility: {}", e)))?;
            
            Ok(result.result.value
                .and_then(|v| v.as_bool())
                .unwrap_or(false))
        }

        WaitCondition::Clickable => {
            // Check visibility + disabled state + pointer-events
            let js_fn = r#"
                function() {
                    const style = window.getComputedStyle(this);
                    if (style.display === 'none' || style.visibility === 'hidden' || style.pointerEvents === 'none') {
                        return false;
                    }
                    if (this.disabled || this.getAttribute('aria-disabled') === 'true') {
                        return false;
                    }
                    const rect = this.getBoundingClientRect();
                    return rect.width > 0 && rect.height > 0;
                }
            "#;
            
            let result = element.call_js_fn(js_fn, false).await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to check clickability: {}", e)))?;
            
            Ok(result.result.value
                .and_then(|v| v.as_bool())
                .unwrap_or(false))
        }

        WaitCondition::TextContains => {
            // Check if element text contains expected string
            let text = args.text.as_ref()
                .ok_or_else(|| McpError::invalid_arguments(
                    "text parameter required for TextContains condition"
                ))?;
            
            let js_fn = format!(
                r#"function() {{
                    const text = (this.innerText || this.textContent || '').trim();
                    return text.includes('{}');
                }}"#,
                text.replace('\\', "\\\\").replace('\'', "\\'")
            );
            
            let result = element.call_js_fn(&js_fn, false).await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to check text: {}", e)))?;
            
            Ok(result.result.value
                .and_then(|v| v.as_bool())
                .unwrap_or(false))
        }

        WaitCondition::AttributeIs => {
            // Check if attribute has specific value
            let attr_name = args.attribute_name.as_ref()
                .ok_or_else(|| McpError::invalid_arguments(
                    "attribute_name parameter required for AttributeIs condition"
                ))?;
            let attr_value = args.attribute_value.as_ref()
                .ok_or_else(|| McpError::invalid_arguments(
                    "attribute_value parameter required for AttributeIs condition"
                ))?;
            
            // Safe: parameterized evaluation prevents injection
            let call = CallFunctionOnParams::builder()
                .function_declaration("(attrName, attrValue) => { return this.getAttribute(attrName) === attrValue; }")
                .object_id(element.remote_object_id.clone())
                .argument(CallArgument::builder().value(json!(attr_name)).build())
                .argument(CallArgument::builder().value(json!(attr_value)).build())
                .build()
                .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to build attribute check params: {}", e)))?;
            
            let result = page.execute(call).await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to check attribute: {}", e)))?;
            
            Ok(result.result.value
                .and_then(|v| v.as_bool())
                .unwrap_or(false))
        }
    }
}
