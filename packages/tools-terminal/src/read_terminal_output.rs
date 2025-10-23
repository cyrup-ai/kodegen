use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_tool::Tool;
use crate::manager::TerminalManager;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageRole, PromptMessageContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

// ============================================================================
// TOOL ARGUMENTS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadTerminalOutputArgs {
    /// Process ID to read output from
    pub pid: u32,
    
    /// Offset for pagination (0 = start, negative = tail from end)
    #[serde(default)]
    pub offset: i64,
    
    /// Maximum lines to return (default: 100)
    #[serde(default = "default_length")]
    pub length: usize,
}

fn default_length() -> usize {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadTerminalOutputPromptArgs {}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct ReadTerminalOutputTool {
    terminal_manager: Arc<TerminalManager>,
}

impl ReadTerminalOutputTool {
    #[must_use]
    pub fn new(terminal_manager: Arc<TerminalManager>) -> Self {
        Self { terminal_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ReadTerminalOutputTool {
    type Args = ReadTerminalOutputArgs;
    type PromptArgs = ReadTerminalOutputPromptArgs;

    fn name() -> &'static str {
        "read_terminal_output"
    }

    fn description() -> &'static str {
        "Get output from a PTY terminal session with offset-based pagination.\n\n\
         Supports partial output reading from VT100 screen buffer:\n\
         - offset: 0, length: 100     → First 100 lines\n\
         - offset: 200, length: 50    → Lines 200-249\n\
         - offset: -20                → Last 20 lines\n\n\
         Output is buffered in VT100 scrollback and can be re-read multiple times.\n\
         Non-destructive - does not clear output."
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let response = self.terminal_manager
            .get_output(args.pid, args.offset, args.length)
            .await
            .ok_or_else(|| McpError::InvalidArguments(
                format!("Terminal session {} not found", args.pid)
            ))?;
        
        // Transform lines from Vec<String> to Vec<{line: usize, output: String}>
        let start_line = if args.offset < 0 {
            response.total_lines.saturating_sub(response.lines.len())
        } else {
            args.offset.max(0) as usize
        };
        
        let formatted_lines: Vec<Value> = response.lines
            .into_iter()
            .enumerate()
            .map(|(idx, io)| json!({
                "line": start_line + idx,
                "io": io
            }))
            .collect();
        
        Ok(json!({
            "pid": response.pid,
            "lines": formatted_lines,
            "total_lines": response.total_lines,
            "lines_returned": response.lines_returned,
            "is_complete": response.is_complete,
            "exit_code": response.exit_code,
            "has_more": response.has_more,
            "buffer_truncated": response.buffer_truncated,
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I read paginated output from a terminal command?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use read_terminal_output with pagination:\n\
                     \n\
                     First page:\n\
                     {\"pid\": 12345, \"offset\": 0, \"length\": 100}\n\
                     \n\
                     Next page:\n\
                     {\"pid\": 12345, \"offset\": 100, \"length\": 100}\n\
                     \n\
                     Last 20 lines:\n\
                     {\"pid\": 12345, \"offset\": -20}\n\
                     \n\
                     Re-read from start:\n\
                     {\"pid\": 12345, \"offset\": 0}\n\
                     \n\
                     The response includes:\n\
                     - lines: Array of output lines for this page\n\
                     - total_lines: Total buffered lines available (max 10,000 via VT100 scrollback)\n\
                     - lines_returned: Count of lines in this response\n\
                     - is_complete: Process has finished\n\
                     - exit_code: Exit status (if complete)\n\
                     - has_more: More output may arrive\n\
                     - buffer_truncated: Indicates if older output was dropped due to scrollback limit\n\
                     \n\
                     Output is persistent and can be read multiple times.\n\
                     Non-destructive - does not consume/clear output."
                ),
            },
        ])
    }
}
