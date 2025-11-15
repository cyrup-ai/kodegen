//! Execution tools: terminal

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn execution_tools() -> Vec<ToolMetadata> {
    vec![
        // TERMINAL (5 tools)
        ToolMetadata {
            name: TERMINAL_LIST_COMMANDS,
            category: "terminal",
            description: "List all active command sessions. Returns array of sessions with PID, blocked status, and runtime. Use this to monitor all running commands and get...",
            schema: build_schema::<terminal::ListTerminalCommandsArgs>(),
        },
        ToolMetadata {
            name: TERMINAL_READ_OUTPUT,
            category: "terminal",
            description: "Get output from a PTY terminal session with offset-based pagination.nn Supports partial output reading from VT100 screen buffer:n - offset: 0, leng...",
            schema: build_schema::<terminal::ReadTerminalOutputArgs>(),
        },
        ToolMetadata {
            name: TERMINAL_SEND_INPUT,
            category: "terminal",
            description: "Send input text to a running PTY terminal process. Perfect for interacting with REPLs (Python, Node.js, etc.), interactive programs (vim, top), and...",
            schema: build_schema::<terminal::SendTerminalInputArgs>(),
        },
        ToolMetadata {
            name: TERMINAL_START_COMMAND,
            category: "terminal",
            description: "Execute a shell command with full terminal emulation. Supports long-running commands, output streaming, and session management. Returns PID for tra...",
            schema: build_schema::<terminal::StartTerminalCommandArgs>(),
        },
        ToolMetadata {
            name: TERMINAL_STOP_COMMAND,
            category: "terminal",
            description: "Force terminate a running command session by PID. Attempts graceful termination first (SIGTERM), then force kills after 1 second if still running (...",
            schema: build_schema::<terminal::StopTerminalCommandArgs>(),
        },
    ]
}
