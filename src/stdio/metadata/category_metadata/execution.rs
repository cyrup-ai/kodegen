//! Execution tools: terminal

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn execution_tools() -> Vec<ToolMetadata> {
    vec![
        // TERMINAL (2 tools)
        ToolMetadata {
            name: TERMINAL_RUN_COMMAND,
            category: "terminal",
            description: "Execute shell command with real-time last-line streaming. Blocks until completion or timeout. Returns complete output and exit code when finished.",
            schema: build_schema::<terminal::TerminalRunCommandArgs>(),
        },
        ToolMetadata {
            name: TERMINAL_SEND_INPUT,
            category: "terminal",
            description: "Send input to running interactive command (REPL, vim, etc). Perfect for interacting with long-running commands started by run_command.",
            schema: build_schema::<terminal::SendTerminalInputArgs>(),
        },
    ]
}
