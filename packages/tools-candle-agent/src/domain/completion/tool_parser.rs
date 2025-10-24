//! Streaming parser for Qwen3 tool calls
//!
//! Detects `<tool_call>{"name": "...", "arguments": {...}}</tool_call>`
//! tags in streaming LLM output.
//!
//! # Design
//! The parser maintains state across token emissions to handle:
//! - Partial tags split across tokens
//! - Incremental JSON accumulation
//! - Multi-token tool call content
//!
//! # Architecture
//! ```text
//! TokenOutputStream → ToolCallParser → CandleCompletionChunk
//!   (text chunks)      (stateful)        (Text or ToolCallComplete)
//! ```
//!
//! # References
//! - Qwen3 Hermes format: <https://qwen.readthedocs.io/en/latest/framework/function_call.html>
//! - Token emission: [`qwen3_quantized.rs:401,501`](../../capability/text_to_text/qwen3_quantized.rs)
//! - Target chunk type: [`completion.rs:50-54`](../../domain/context/chunks/completion.rs)

use serde::{Deserialize, Serialize};

/// Parsed tool call from LLM output
///
/// This type is converted to `CandleCompletionChunk::ToolCallComplete` by the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name to execute
    pub name: String,
    /// Tool arguments as JSON string
    pub arguments: String,
}

/// Stateful parser for detecting tool calls in token stream
///
/// # State Machine
/// ```text
/// IDLE ──<tool_call>──► ACCUMULATING ──</tool_call>──► PARSE ──► IDLE
///   │                        │                            │
///   └─ return None ──────────┴─ accumulate ──────────────┴─ return Some(ToolCall)
/// ```
///
/// # Usage
/// ```rust
/// let mut parser = ToolCallParser::new();
///
/// // Process each token from the stream
/// for token in stream {
///     if let Some(tool_call) = parser.process_token(&token) {
///         // Complete tool call detected - emit ToolCallComplete chunk
///         execute_tool(&tool_call.name, &tool_call.arguments);
///     } else {
///         // Regular text - emit Text chunk
///     }
/// }
/// ```
///
/// # Performance
/// - Buffer size: 2-3 strings (~100 bytes overhead)
/// - String operations: `push_str()`, `contains()`, `find()` - all O(n) where n is buffer length
/// - Negligible compared to model inference (~100ms per token)
#[derive(Debug, Default)]
pub struct ToolCallParser {
    /// Buffer for accumulating text between tokens
    ///
    /// Holds partial tags like "<tool" or "}" that haven't been classified yet.
    buffer: String,

    /// Whether currently inside `<tool_call>` tag
    ///
    /// State machine flag: false = IDLE, true = ACCUMULATING
    in_tool_call: bool,

    /// Accumulated JSON content inside tag
    ///
    /// Contains everything between `<tool_call>` and `</tool_call>`
    tool_call_content: String,
}

impl ToolCallParser {
    /// Create new parser
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a new token from the LLM
    ///
    /// # Arguments
    /// * `token` - Text token from LLM generation (from `TokenOutputStream`)
    ///
    /// # Returns
    /// * `Some(ToolCall)` - When a complete tool call is detected and successfully parsed
    /// * `None` - Still accumulating or no tool call in this token
    ///
    /// # State Transitions
    /// 1. IDLE + detect `<tool_call>` → ACCUMULATING (clear content, start buffering)
    /// 2. ACCUMULATING + no `</tool_call>` → ACCUMULATING (append to content)
    /// 3. ACCUMULATING + detect `</tool_call>` → PARSE → IDLE (return `ToolCall`)
    pub fn process_token(&mut self, token: &str) -> Option<ToolCall> {
        self.buffer.push_str(token);

        // Opening tag detection
        if !self.in_tool_call && self.buffer.contains("<tool_call>") {
            self.in_tool_call = true;
            self.tool_call_content.clear();

            // Extract content after opening tag
            // Example: buffer = "text <tool_call>{\"na" → content = "{\"na"
            if let Some(idx) = self.buffer.find("<tool_call>") {
                let after_tag = &self.buffer[idx + "<tool_call>".len()..];
                self.tool_call_content.push_str(after_tag);
                self.buffer.clear();
            }
            return None;
        }

        // Accumulate content while inside tag
        if self.in_tool_call && !self.buffer.contains("</tool_call>") {
            // Example: token = "me\": \"r" → content += "me\": \"r"
            self.tool_call_content.push_str(token);
            return None;
        }

        // Closing tag detection
        if self.in_tool_call && self.buffer.contains("</tool_call>") {
            // Extract content before closing tag
            // Example: buffer = "...}}</tool_call>" → content += "...}}"
            if let Some(idx) = self.buffer.find("</tool_call>") {
                let before_tag = &self.buffer[..idx];
                self.tool_call_content.push_str(before_tag);
            }

            // Parse the complete JSON
            let result = Self::parse_tool_call_json(&self.tool_call_content);

            // Reset state for next tool call
            self.in_tool_call = false;
            self.tool_call_content.clear();
            self.buffer.clear();

            return result;
        }

        None
    }

    /// Parse tool call JSON: `{"name": "tool_name", "arguments": {...}}`
    ///
    /// # Format
    /// Expected JSON structure from Qwen3:
    /// ```json
    /// {
    ///   "name": "read_file",
    ///   "arguments": {
    ///     "path": "/tmp/test.txt"
    ///   }
    /// }
    /// ```
    ///
    /// # Error Handling
    /// - Invalid JSON: Log warning and return None (model may generate malformed JSON)
    /// - Missing fields: Return None via `?` operator
    /// - Malformed arguments: Return None (router will handle validation)
    fn parse_tool_call_json(json_str: &str) -> Option<ToolCall> {
        let trimmed = json_str.trim();

        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(json) => {
                // Extract "name" field
                let name = json["name"].as_str()?.to_string();

                // Extract "arguments" field and serialize back to JSON string
                // This preserves the structure for router.call_tool()
                let arguments = json["arguments"].clone();
                let args_string = serde_json::to_string(&arguments).ok()?;

                Some(ToolCall {
                    name,
                    arguments: args_string,
                })
            }
            Err(e) => {
                // Model may generate invalid JSON - log and continue
                log::warn!("Failed to parse tool call JSON: {e}");
                None
            }
        }
    }

    /// Reset parser state
    ///
    /// Call this when starting a new generation to clear any accumulated state.
    /// Useful for reusing the parser across multiple inference cycles.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.in_tool_call = false;
        self.tool_call_content.clear();
    }
}
