//! Browser extract text tool - gets page or element text content

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptMessage, PromptMessageRole, PromptMessageContent, PromptArgument};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::manager::BrowserManager;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserExtractTextArgs {
    /// Optional: CSS selector for specific element (default: entire page)
    #[serde(default)]
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserExtractTextPromptArgs {}

#[derive(Clone)]
pub struct BrowserExtractTextTool {
    manager: Arc<BrowserManager>,
}

impl BrowserExtractTextTool {
    pub fn new(manager: Arc<BrowserManager>) -> Self {
        Self { manager }
    }
}

#[async_trait::async_trait]
impl Tool for BrowserExtractTextTool {
    type Args = BrowserExtractTextArgs;
    type PromptArgs = BrowserExtractTextPromptArgs;

    fn name() -> &'static str {
        "browser_extract_text"
    }

    fn description() -> &'static str {
        "Extract text content from the page or specific element.\\n\\n\
         Returns the text content for AI agent analysis.\\n\\n\
         Example: browser_extract_text({}) for full page\\n\
         Example: browser_extract_text({\"selector\": \"#content\"}) for specific element"
    }

    fn read_only() -> bool {
        true // Extraction doesn't modify page
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Get browser context
        let context = self.manager.get_or_create_context().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Browser error: {}", e)))?;

        // Get current page
        let page = context.get_current_page().await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get page: {}", e)))?;

        // Extract text
        let text = if let Some(selector) = &args.selector {
            // Extract from specific element
            page.text_content(selector).await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to get text: {}", e)))?
        } else {
            // Extract from entire page - get body text
            let body_text = page.evaluate("document.body.innerText").await
                .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to extract page text: {}", e)))?;

            body_text.value()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        Ok(json!({
            "success": true,
            "text": text,
            "length": text.len(),
            "selector": args.selector,
            "message": format!("Extracted {} characters", text.len())
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I get text from a page?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_extract_text to get page content. Examples:\\n\
                     - browser_extract_text({}) - Full page text\\n\
                     - browser_extract_text({\"selector\": \"#article\"}) - Specific element\\n\
                     - browser_extract_text({\"selector\": \".content\"}) - By class"
                ),
            },
        ])
    }
}
