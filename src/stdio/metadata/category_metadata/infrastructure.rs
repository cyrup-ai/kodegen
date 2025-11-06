//! Infrastructure tools: config, introspection, process

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn infrastructure_tools() -> Vec<ToolMetadata> {
    vec![
        // CONFIG (2 tools)
        ToolMetadata {
            name: "get_config",
            category: "config",
            description: "Get complete server configuration including security settings (blocked commands, allowed directories), shell preferences, resource limits, and live...",
            schema: build_schema::<config::GetConfigArgs>(),
        },
        ToolMetadata {
            name: "set_config_value",
            category: "config",
            description: "Set a specific configuration value by key.nn WARNING: Should be used in a separate chat from file operations and n command execution to prevent sec...",
            schema: build_schema::<config::SetConfigValueArgs>(),
        },
        // INTROSPECTION (2 tools)
        ToolMetadata {
            name: "get_recent_tool_calls",
            category: "introspection",
            description: "Get recent tool call history with their arguments and outputs. Returns chronological list of tool calls made during this session. Supports paginati...",
            schema: build_schema::<introspection::GetRecentToolCallsArgs>(),
        },
        ToolMetadata {
            name: "get_usage_stats",
            category: "introspection",
            description: "Get usage statistics for debugging and analysis. Returns summary of tool usage, success/failure rates, and performance metrics.' } async fn execute...",
            schema: build_schema::<introspection::GetUsageStatsArgs>(),
        },
        // PROCESS (2 tools)
        ToolMetadata {
            name: "kill_process",
            category: "process",
            description: "Terminate a running process by its PID. Sends SIGKILL signal to forcefully stop the process. Use with caution as this does not allow graceful shutd...",
            schema: build_schema::<process::KillProcessArgs>(),
        },
        ToolMetadata {
            name: "list_processes",
            category: "process",
            description: "List all running processes with PID, command name, CPU usage, and memory usage. Supports filtering by process name and limiting results. Returns co...",
            schema: build_schema::<process::ListProcessesArgs>(),
        },
    ]
}
