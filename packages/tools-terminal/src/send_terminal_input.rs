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
pub struct SendTerminalInputArgs {
    /// Process ID to send input to
    pub pid: u32,
    
    /// Input text to send
    pub input: String,
    
    /// Append newline to execute command (default: true)
    /// Set to false for raw input like Ctrl+C or partial commands
    #[serde(default = "default_append_newline")]
    pub append_newline: bool,
}

fn default_append_newline() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendTerminalInputPromptArgs {}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct SendTerminalInputTool {
    terminal_manager: Arc<TerminalManager>,
}

impl SendTerminalInputTool {
    #[must_use]
    pub fn new(terminal_manager: Arc<TerminalManager>) -> Self {
        Self { terminal_manager }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for SendTerminalInputTool {
    type Args = SendTerminalInputArgs;
    type PromptArgs = SendTerminalInputPromptArgs;

    fn name() -> &'static str {
        "send_terminal_input"
    }

    fn description() -> &'static str {
        "Send input text to a running PTY terminal process. Perfect for interacting with REPLs \
         (Python, Node.js, etc.), interactive programs (vim, top), and responding to prompts. \
         Automatically adds newline for command execution."
    }

    fn read_only() -> bool {
        false
    }

    fn idempotent() -> bool {
        false  // Each input execution has cumulative effect in REPL state
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let success = self.terminal_manager
            .send_input(args.pid, &args.input, args.append_newline)
            .await?;
        
        Ok(json!({
            "success": success,
            "pid": args.pid,
            "input_sent": args.input,
            "newline_appended": args.append_newline
        }))
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        let messages = vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I send Ctrl+C to interrupt a running command?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    r#"To send control characters and raw input, use append_newline parameter:

SENDING CTRL+C (INTERRUPT):
{"tool": "send_terminal_input", "args": {"pid": 12345, "input": "\u0003", "append_newline": false}}

SENDING REGULAR COMMAND (AUTO-NEWLINE):
{"tool": "send_terminal_input", "args": {"pid": 12345, "input": "ls -la"}}
// Equivalent to:
{"tool": "send_terminal_input", "args": {"pid": 12345, "input": "ls -la", "append_newline": true}}

CONTROL CHARACTER REFERENCE:
- Ctrl+C (interrupt): "\u0003" (ASCII 3)
- Ctrl+D (EOF): "\u0004" (ASCII 4)
- Ctrl+Z (suspend): "\u001A" (ASCII 26)
- ESC: "\u001B" (ASCII 27)

PARTIAL INPUT (NO EXECUTION):
{"tool": "send_terminal_input", "args": {"pid": 12345, "input": "def hello():", "append_newline": false}}

EXAMPLE: STOP RUNAWAY PYTHON SCRIPT
1. Start Python:
   {"tool": "start_terminal_command", "args": {"command": "python3 -c 'while True: pass'"}}
   → Returns {"pid": 12345}

2. Send Ctrl+C to interrupt:
   {"tool": "send_terminal_input", "args": {"pid": 12345, "input": "\u0003", "append_newline": false}}

3. Verify stopped:
   {"tool": "read_terminal_output", "args": {"pid": 12345}}
   → Should show KeyboardInterrupt

BACKWARD COMPATIBILITY:
Default behavior unchanged - newline appended automatically unless append_newline=false.
"#
                ),
            },
        ];
        Ok(messages)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }
}
