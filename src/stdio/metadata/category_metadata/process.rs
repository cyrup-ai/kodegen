//! Process management tools: list and kill processes

use kodegen_mcp_schema::process;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn process_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: process::PROCESS_KILL,
            category: "process",
            description: "Terminate a running process by its PID. Sends SIGKILL signal to forcefully stop the process. Use with caution as this does not allow graceful shutd...",
            schema: build_schema::<process::ProcessKillArgs>(),
        },
        ToolMetadata {
            name: process::PROCESS_LIST,
            category: "process",
            description: "List all running processes with PID, command name, CPU usage, and memory usage. Supports filtering by process name and limiting results. Returns co...",
            schema: build_schema::<process::ProcessListArgs>(),
        },
    ]
}
