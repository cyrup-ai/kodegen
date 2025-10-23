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
pub struct ListTerminalCommandsArgs {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTerminalCommandsPromptArgs {}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct ListTerminalCommandsTool {
    terminal_manager: Arc<TerminalManager>,
}

impl ListTerminalCommandsTool {
    #[must_use]
    pub fn new(terminal_manager: Arc<TerminalManager>) -> Self {
        Self { terminal_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ListTerminalCommandsTool {
    type Args = ListTerminalCommandsArgs;
    type PromptArgs = ListTerminalCommandsPromptArgs;

    fn name() -> &'static str {
        "list_terminal_commands"
    }

    fn description() -> &'static str {
        "List all active command sessions. Returns array of sessions with PID, blocked status, \
         and runtime. Use this to monitor all running commands and get PIDs for read_terminal_output or \
         stop_terminal_command operations."
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

    async fn execute(&self, _args: Self::Args) -> Result<Value, McpError> {
        // Get active sessions
        let active_sessions = self.terminal_manager.list_active_sessions();

        // Format the response
        if active_sessions.is_empty() {
            Ok(json!({
                "sessions": [],
                "count": 0,
                "message": "No active command sessions. All commands have completed."
            }))
        } else {
            // Convert runtime from milliseconds to seconds for better readability
            let formatted_sessions: Vec<Value> = active_sessions
                .iter()
                .map(|session| {
                    // Convert runtime to seconds for display
                    // Runtime < 2^52 ms for all realistic sessions
                    let runtime_s = (session.runtime as f64) / 1000.0;
                    
                    json!({
                        "pid": session.pid,
                        "is_blocked": session.is_blocked,
                        "runtime_ms": session.runtime,
                        "runtime_s": format!("{runtime_s:.2}"),
                    })
                })
                .collect();

            Ok(json!({
                "sessions": formatted_sessions,
                "count": active_sessions.len(),
                "message": format!(
                    "{} active session(s). Use read_terminal_output(pid) to get output or stop_terminal_command(pid) to stop.",
                    active_sessions.len()
                )
            }))
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I see all my running commands?"
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The list_commands tool shows all active command sessions:\n\n\
                     Basic usage:\n\
                     list_commands({})\n\n\
                     Returns:\n\
                     {\n\
                       \"sessions\": [\n\
                         {\n\
                           \"pid\": 12345,\n\
                           \"is_blocked\": true,\n\
                           \"runtime_ms\": 5430,\n\
                           \"runtime_s\": \"5.43\"\n\
                         },\n\
                         {\n\
                           \"pid\": 12346,\n\
                           \"is_blocked\": false,\n\
                           \"runtime_ms\": 1200,\n\
                           \"runtime_s\": \"1.20\"\n\
                         }\n\
                       ],\n\
                       \"count\": 2,\n\
                       \"message\": \"2 active session(s)...\"\n\
                     }\n\n\
                     Understanding the output:\n\
                     - pid: Process ID to use with read_terminal_output or stop_terminal_command\n\
                     - is_blocked: true = long-running (timed out), false = about to complete\n\
                     - runtime_ms: How long the process has been running (milliseconds)\n\
                     - runtime_s: Human-readable runtime (seconds)\n\n\
                     Common workflows:\n\
                     1. Check what's running:\n\
                        list_commands() → See all PIDs\n\
                     2. Get output from specific session:\n\
                        read_terminal_output({\"pid\": 12345})\n\
                     3. Stop unwanted session:\n\
                        stop_terminal_command({\"pid\": 12345})\n\n\
                     When to use:\n\
                     - Lost track of running commands\n\
                     - Want to monitor all active processes\n\
                     - Need PIDs for other operations\n\
                     - Checking if long-running command is still active\n\n\
                     Best practices:\n\
                     - Call periodically to monitor long-running operations\n\
                     - Use with read_terminal_output to process multiple commands\n\
                     - Check before starting new commands to avoid overload\n\n\
                     Example multi-session management:\n\
                     1. start_terminal_command({\"command\": \"npm install\"}) → pid 100\n\
                     2. start_terminal_command({\"command\": \"cargo build\"}) → pid 101\n\
                     3. list_terminal_commands() → [{pid: 100, ...}, {pid: 101, ...}]\n\
                     4. read_terminal_output({\"pid\": 100}) → Check npm progress\n\
                     5. read_terminal_output({\"pid\": 101}) → Check cargo progress\n\
                     6. stop_terminal_command({\"pid\": 100}) → Cancel npm if needed"
                ),
            },
        ])
    }
}
