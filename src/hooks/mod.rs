pub mod notify;

use serde::Deserialize;

// Re-export the actual schema types from kodegen-mcp-schema
pub use kodegen_mcp_schema::terminal::{TerminalInput, TerminalOutput};

/// MCP Content - tagged enum matching rmcp structure
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text {
        text: String,
    },
    #[serde(other)]
    Other,
}

/// MCP CallToolResult - structure of tool_response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<Content>,
    #[serde(default)]
    pub is_error: Option<bool>,
}

/// Hook input from Claude Code - matches the JSON sent to stdin
/// See: https://docs.anthropic.com/en/docs/claude-code/hooks
#[derive(Debug, Deserialize)]
pub struct HookInput {
    /// Session identifier
    pub session_id: String,

    /// Path to the transcript JSONL file
    pub transcript_path: String,

    /// Current working directory
    pub cwd: String,

    /// Hook event name: "PostToolUse", "SessionEnd", etc.
    pub hook_event_name: String,

    // PostToolUse-specific fields
    /// Tool name (e.g., "mcp__plugin_kodegen_kodegen__terminal", "Write", "Read")
    #[serde(default)]
    pub tool_name: Option<String>,

    /// Tool input - deserialize based on tool_name
    #[serde(default)]
    pub tool_input: Option<serde_json::Value>,

    /// Tool response - MCP CallToolResult structure
    #[serde(default)]
    pub tool_response: Option<CallToolResult>,

    // SessionEnd-specific fields
    /// Reason for session end
    #[serde(default)]
    pub reason: Option<String>,
}

impl HookInput {
    /// Check if tool response is an error
    pub fn is_tool_error(&self) -> bool {
        self.tool_response
            .as_ref()
            .and_then(|r| r.is_error)
            .unwrap_or(false)
    }

    /// Get error message from tool_response (when is_error = true)
    pub fn error_message(&self) -> Option<String> {
        if !self.is_tool_error() {
            return None;
        }
        self.tool_response.as_ref().and_then(|result| {
            result.content.first().and_then(|content| {
                if let Content::Text { text } = content {
                    Some(text.clone())
                } else {
                    None
                }
            })
        })
    }

    /// Extract TerminalOutput from tool_response.content[0].text (when NOT an error)
    pub fn terminal_output(&self) -> Option<TerminalOutput> {
        if self.is_tool_error() {
            return None;
        }
        self.tool_response.as_ref().and_then(|result| {
            result.content.first().and_then(|content| {
                if let Content::Text { text } = content {
                    serde_json::from_str(text).ok()
                } else {
                    None
                }
            })
        })
    }

    /// Extract TerminalInput from tool_input
    pub fn terminal_input(&self) -> Option<TerminalInput> {
        self.tool_input
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Read hook input from stdin
pub fn read_hook_input() -> anyhow::Result<HookInput> {
    let stdin = std::io::stdin();
    Ok(serde_json::from_reader(stdin)?)
}
