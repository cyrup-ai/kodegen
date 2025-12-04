pub mod notify;
pub mod stop;

use serde::Deserialize;

// Re-export the actual schema types from kodegen-mcp-schema
pub use kodegen_mcp_schema::terminal::{TerminalInput, TerminalOutput};
pub use kodegen_mcp_schema::{deserialize_typed_only, AnyToolOutput};

/// PostToolUse hook input from Claude Code
/// See: https://docs.anthropic.com/en/docs/claude-code/hooks
#[derive(Debug, Deserialize)]
pub struct PostToolUseInput {
    /// Session identifier
    pub session_id: String,

    /// Path to the transcript JSONL file
    pub transcript_path: String,

    /// Current working directory
    pub cwd: String,

    /// Permission mode: "default", "plan", "acceptEdits", or "bypassPermissions"
    pub permission_mode: String,

    /// Hook event name (should be "PostToolUse")
    pub hook_event_name: String,

    /// Tool name (e.g., "mcp__plugin_kodegen_kodegen__terminal", "Write", "Read")
    pub tool_name: String,

    /// Tool input - raw JSON (varies by tool)
    pub tool_input: serde_json::Value,

    /// Tool response - raw JSON with success field and tool-specific fields
    pub tool_response: serde_json::Value,

    /// Tool use ID from Claude
    pub tool_use_id: String,
}

/// Stop hook input from Claude Code
/// See: https://docs.anthropic.com/en/docs/claude-code/hooks
#[derive(Debug, Deserialize)]
pub struct StopInput {
    /// Session identifier
    pub session_id: String,

    /// Path to the transcript JSONL file
    pub transcript_path: String,

    /// Permission mode: "default", "plan", "acceptEdits", or "bypassPermissions"
    pub permission_mode: String,

    /// Hook event name (should be "Stop")
    pub hook_event_name: String,

    /// True when Claude Code is already continuing as a result of a stop hook
    pub stop_hook_active: bool,
}

impl PostToolUseInput {
    /// Check if this is a kodegen MCP tool event
    pub fn is_kodegen_tool(&self) -> bool {
        self.tool_name.starts_with("mcp__plugin_kodegen_kodegen__")
    }

    /// Get canonical tool name (strip MCP prefix)
    /// Returns Some("terminal") for "mcp__plugin_kodegen_kodegen__terminal"
    pub fn canonical_tool_name(&self) -> Option<&str> {
        self.tool_name.strip_prefix("mcp__plugin_kodegen_kodegen__")
    }

    /// Check if tool errored (success field is false)
    pub fn is_tool_error(&self) -> bool {
        self.tool_response
            .get("success")
            .and_then(|v| v.as_bool())
            .map(|success| !success)
            .unwrap_or(false)
    }

    /// Deserialize tool_response into our AnyToolOutput enum
    ///
    /// This handles the fact that Claude Code adds a `success` field to tool responses.
    /// Our schema types don't have this field, but serde ignores unknown fields by default.
    pub fn typed_output(&self) -> Option<AnyToolOutput> {
        let canonical_name = self.canonical_tool_name()?;
        let json_str = serde_json::to_string(&self.tool_response).ok()?;
        deserialize_typed_only(canonical_name, &json_str).ok()
    }

    /// Get terminal output if this is a terminal tool
    pub fn terminal_output(&self) -> Option<TerminalOutput> {
        match self.typed_output()? {
            AnyToolOutput::Terminal(output) => Some(output),
            _ => None,
        }
    }

    /// Extract TerminalInput from tool_input
    pub fn terminal_input(&self) -> Option<TerminalInput> {
        serde_json::from_value(self.tool_input.clone()).ok()
    }
}
