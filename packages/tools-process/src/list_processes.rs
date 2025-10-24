use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::process::{ListProcessesArgs, ListProcessesPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};
use sysinfo::System;

use crate::ProcessId;

// ============================================================================
// SHARED TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProcessInfo {
    /// Process ID
    pub pid: ProcessId,

    /// Process name/command
    pub name: String,

    /// CPU usage percentage
    pub cpu_percent: f32,

    /// Memory usage in MB
    pub memory_mb: f64,
}

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone, Default)]
pub struct ListProcessesTool;

impl ListProcessesTool {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for ListProcessesTool {
    type Args = ListProcessesArgs;
    type PromptArgs = ListProcessesPromptArgs;

    fn name() -> &'static str {
        "list_processes"
    }

    fn description() -> &'static str {
        "List all running processes with PID, command name, CPU usage, and memory usage. \
         Supports filtering by process name and limiting results. Returns comprehensive \
         process information for system monitoring and debugging."
    }

    fn read_only() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Clone filter before moving args into closure
        let filter_clone = args.filter.clone();
        let limit = args.limit;

        // Use spawn_blocking because sysinfo operations are CPU-intensive
        let processes = tokio::task::spawn_blocking(move || {
            let mut system = System::new_all();
            system.refresh_all();

            let mut process_list: Vec<ProcessInfo> = system
                .processes()
                .iter()
                .map(|(pid, process)| {
                    ProcessInfo {
                        pid: pid.as_u32(),
                        name: process.name().to_string_lossy().to_string(),
                        cpu_percent: process.cpu_usage(),
                        // Note: Precision loss is acceptable for display purposes
                        memory_mb: f64::from(u32::try_from(process.memory()).unwrap_or(u32::MAX))
                            / 1024.0
                            / 1024.0,
                    }
                })
                .collect();

            // Apply filter if provided
            if let Some(filter) = &args.filter {
                let filter_lower = filter.to_lowercase();
                process_list.retain(|p| p.name.to_lowercase().contains(&filter_lower));
            }

            // Sort by CPU usage (descending) for useful output
            process_list.sort_by(|a, b| {
                b.cpu_percent
                    .partial_cmp(&a.cpu_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Apply limit if specified
            if args.limit > 0 {
                process_list.truncate(args.limit);
            }

            process_list
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to list processes: {e}")))?;

        Ok(json!({
            "processes": processes,
            "total_count": processes.len(),
            "filter": filter_clone,
            "limited": limit > 0 && processes.len() >= limit
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I list running processes?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The list_processes tool shows all running processes:\n\n\
                     1. List all processes:\n\
                        list_processes({})\n\n\
                     2. Filter by name:\n\
                        list_processes({\"filter\": \"python\"})\n\n\
                     3. Limit results:\n\
                        list_processes({\"filter\": \"node\", \"limit\": 10})\n\n\
                     Returns for each process:\n\
                     - pid: Process ID\n\
                     - name: Process/command name\n\
                     - cpu_percent: CPU usage percentage\n\
                     - memory_mb: Memory usage in megabytes\n\n\
                     Processes are sorted by CPU usage (highest first) for easy \
                     identification of resource-heavy processes.\n\n\
                     Use this for:\n\
                     - System monitoring\n\
                     - Finding PIDs for processes to terminate\n\
                     - Debugging performance issues\n\
                     - Checking if a specific process is running",
                ),
            },
        ])
    }
}
