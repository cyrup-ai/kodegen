use crate::manager::{CommandManager, TerminalManager};
use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

// ============================================================================
// TOOL ARGUMENTS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartTerminalCommandArgs {
    /// The shell command to execute
    pub command: String,

    /// Initial delay in milliseconds before returning first response (default: 100ms)
    /// Allows quick commands (pwd, echo) to complete before returning
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// Shell to use (optional, defaults to system shell)
    #[serde(default)]
    pub shell: Option<String>,
}

fn default_initial_delay() -> u64 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartTerminalCommandPromptArgs {}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone)]
pub struct StartTerminalCommandTool {
    terminal_manager: Arc<TerminalManager>,
    command_manager: CommandManager,
}

impl StartTerminalCommandTool {
    #[must_use]
    pub fn new(terminal_manager: Arc<TerminalManager>, command_manager: CommandManager) -> Self {
        Self {
            terminal_manager,
            command_manager,
        }
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for StartTerminalCommandTool {
    type Args = StartTerminalCommandArgs;
    type PromptArgs = StartTerminalCommandPromptArgs;

    fn name() -> &'static str {
        "start_terminal_command"
    }

    fn description() -> &'static str {
        "Execute a shell command with full terminal emulation. Supports long-running commands, \
         output streaming, and session management. Returns PID for tracking and initial output. \
         Use read_terminal_output to get more output from long-running commands."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true
    }

    fn open_world() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Validate command against blocked list
        let is_allowed = self.command_manager.validate_command(&args.command);
        if !is_allowed {
            return Err(McpError::PermissionDenied(format!(
                "Command not allowed: {}. This command is in the blocked list for safety.",
                args.command
            )));
        }

        // Execute via terminal manager
        let result = self
            .terminal_manager
            .execute_command(
                &args.command,
                Some(args.initial_delay_ms),
                args.shell.as_deref(),
            )
            .await
            .map_err(McpError::Other)?;

        // Format response
        Ok(json!({
            "pid": result.pid,
            "output": result.output,
            "is_blocked": result.is_blocked,
            "ready_for_input": result.ready_for_input,
            "message": if result.ready_for_input {
                format!(
                    "REPL ready for input (PID: {}). Use send_input({{\"pid\": {}, \"input\": \"...\"}}) to interact.",
                    result.pid, result.pid
                )
            } else if result.is_blocked {
                format!(
                    "Command still running (PID: {}). Use read_output({{\"pid\": {}}}) to get more output.",
                    result.pid, result.pid
                )
            } else {
                "Command completed.".to_string()
            }
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I execute shell commands?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The start_terminal_command tool runs shell commands with full terminal support:\n\n\
                     Basic usage:\n\
                     start_terminal_command({\"command\": \"ls -la\"})\n\n\
                     With custom initial delay (waits longer before returning):\n\
                     start_terminal_command({\"command\": \"npm install\", \"initial_delay_ms\": 1000})\n\n\
                     With specific shell:\n\
                     start_terminal_command({\"command\": \"echo $SHELL\", \"shell\": \"/bin/bash\"})\n\n\
                     Key features:\n\
                     - Full PTY support for interactive commands\n\
                     - Session tracking with PID for long-running commands\n\
                     - Output streaming (use read_terminal_output for more output)\n\
                     - Default initial_delay_ms is 100ms (brief wait for quick commands)\n\
                     - Command validation for safety (blocks dangerous commands)\n\n\
                     For long-running commands:\n\
                     1. start_terminal_command returns PID after initial_delay_ms\n\
                     2. Command continues running in background\n\
                     3. Use read_terminal_output({\"pid\": <pid>}) to get ongoing output\n\
                     4. Use stop_terminal_command({\"pid\": <pid>}) to stop if needed\n\
                     5. Use list_terminal_commands() to see all active sessions\n\n\
                     Security:\n\
                     - Blocked commands: rm, sudo, chmod, kill, wget, curl, etc.\n\
                     - Complex command parsing handles pipes, redirects, subshells\n\
                     - Safe error handling throughout",
                ),
            },
        ])
    }
}
