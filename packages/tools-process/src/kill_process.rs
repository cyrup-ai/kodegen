use kodegen_mcp_tool::Tool;
use kodegen_mcp_tool::error::McpError;
use kodegen_mcp_schema::process::{KillProcessArgs, KillProcessPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};

// Compile-time platform validation for PID conversion safety
// This ensures u32 → usize conversion cannot truncate
#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("KillProcessTool only supports 32-bit and 64-bit platforms");

// ============================================================================
// TOOL STRUCT
// ============================================================================

#[derive(Clone, Default)]
pub struct KillProcessTool;

impl KillProcessTool {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

impl Tool for KillProcessTool {
    type Args = KillProcessArgs;
    type PromptArgs = KillProcessPromptArgs;

    fn name() -> &'static str {
        "kill_process"
    }

    fn description() -> &'static str {
        "Terminate a running process by its PID. Sends SIGKILL signal to forcefully stop the \
         process. Use with caution as this does not allow graceful shutdown. Returns success \
         if process was terminated, error if process not found or permission denied."
    }

    fn read_only() -> bool {
        false // Modifies system state
    }

    fn destructive() -> bool {
        true // Terminates processes
    }

    fn idempotent() -> bool {
        false // Killing twice will fail (process no longer exists)
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        let pid = args.pid;

        // Validate PID
        if pid == 0 {
            return Err(McpError::InvalidArguments(
                "Invalid PID 0: cannot kill process with ID 0".to_string(),
            ));
        }

        // Use spawn_blocking for sysinfo operations
        let result = tokio::task::spawn_blocking(move || {
            let mut system = System::new();
            system.refresh_processes(ProcessesToUpdate::All, true);

            // Platform-validated PID conversion
            // Safe: u32 fits in usize on 32-bit and 64-bit platforms
            #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
            let sysinfo_pid = {
                // Document why conversion is safe on supported platforms:
                // - On 64-bit: usize = u64, conversion u32 → u64 is lossless
                // - On 32-bit: usize = u32, conversion u32 → u32 is identity
                Pid::from(pid as usize)
            };

            #[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
            {
                return Err("Process termination not supported on this platform");
            }

            if let Some(process) = system.process(sysinfo_pid) {
                let process_name = process.name().to_string_lossy().to_string();
                let killed = process.kill_with(Signal::Kill);

                match killed {
                    Some(true) => Ok(process_name),
                    Some(false) => Err("Permission denied or process protected"),
                    None => Err("Failed to send kill signal"),
                }
            } else {
                Err("Process not found")
            }
        })
        .await
        .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to kill process: {e}")))?;

        match result {
            Ok(process_name) => Ok(json!({
                "success": true,
                "pid": pid,
                "process_name": process_name,
                "message": format!("Successfully terminated process {pid} ({process_name})")
            })),
            Err(reason) => Err(McpError::PermissionDenied(format!(
                "Failed to kill process {pid}: {reason}"
            ))),
        }
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I kill a process?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The kill_process tool terminates a process by PID:\n\n\
                     Usage: kill_process({\"pid\": 12345})\n\n\
                     ⚠️  IMPORTANT - This is DESTRUCTIVE:\n\
                     - Sends SIGKILL (force kill, immediate termination)\n\
                     - Process cannot cleanup or save state\n\
                     - No graceful shutdown\n\
                     - Use with caution!\n\n\
                     Before killing:\n\
                     1. Use list_processes to find the PID\n\
                     2. Verify it's the correct process\n\
                     3. Consider if force kill is necessary\n\n\
                     Error cases:\n\
                     - Process not found: PID doesn't exist\n\
                     - Permission denied: Insufficient privileges (system processes, other users)\n\
                     - Protected process: OS prevents termination\n\n\
                     Returns:\n\
                     - success: true if terminated\n\
                     - pid: The terminated process ID\n\
                     - process_name: Name of the terminated process\n\n\
                     Best practice: Always confirm PID before killing!",
                ),
            },
        ])
    }
}
