use anyhow::Result;
use extism::convert::Json;
use extism::{PluginBuilder, UserData, ValType, host_fn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Deserialize, Serialize)]
pub struct ShellExecuteRequest {
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShellExecuteResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub is_error: bool,
}

pub struct ShellExecutor {
    timeout_duration: Duration,
    blocked_patterns: Vec<Regex>,
    allowed_commands: Option<Vec<String>>,
}

impl Default for ShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellExecutor {
    #[must_use]
    pub fn new() -> Self {
        let mut blocked_patterns = Vec::new();

        // Dangerous recursive deletion
        if let Ok(pattern) = Regex::new(r"rm\s+(-[rfRF]*\s+)*/*\s*$") {
            blocked_patterns.push(pattern);
        }
        if let Ok(pattern) = Regex::new(r"rm\s+(-[rfRF]*\s+)*/\s*$") {
            blocked_patterns.push(pattern);
        }

        // Fork bombs
        if let Ok(pattern) = Regex::new(r":\(\)\s*\{") {
            blocked_patterns.push(pattern);
        }
        if let Ok(pattern) = Regex::new(r"\|\s*:\s*&") {
            blocked_patterns.push(pattern);
        }

        // Command injection attempts
        if let Ok(pattern) = Regex::new(r"`.*`") {
            blocked_patterns.push(pattern);
        }
        if let Ok(pattern) = Regex::new(r"\$\(.*\)") {
            blocked_patterns.push(pattern);
        }

        Self {
            timeout_duration: Duration::from_secs(30),
            blocked_patterns,
            allowed_commands: None, // None = allow all (use blocklist)
        }
    }

    fn validate_command(&self, cmd: &str) -> Result<(), String> {
        // Check blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(cmd) {
                return Err(format!("Command blocked by security policy: {cmd}"));
            }
        }

        // Check whitelist if configured
        if let Some(allowed) = &self.allowed_commands {
            let cmd_base = cmd.split_whitespace().next().unwrap_or("");
            if !allowed.contains(&cmd_base.to_string()) {
                return Err(format!("Command not in whitelist: {cmd_base}"));
            }
        }

        Ok(())
    }

    pub async fn execute(&self, command: &str) -> ShellExecuteResponse {
        // Validate first
        if let Err(e) = self.validate_command(command) {
            return ShellExecuteResponse {
                stdout: String::new(),
                stderr: e,
                exit_code: Some(1),
                is_error: true,
            };
        }

        // Execute with timeout
        let child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let child = match child {
            Ok(c) => c,
            Err(e) => {
                return ShellExecuteResponse {
                    stdout: String::new(),
                    stderr: format!("Failed to spawn process: {e}"),
                    exit_code: Some(1),
                    is_error: true,
                };
            }
        };

        let wait_future = async { child.wait_with_output() };

        let output = match timeout(self.timeout_duration, wait_future).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return ShellExecuteResponse {
                    stdout: String::new(),
                    stderr: format!("Process execution failed: {e}"),
                    exit_code: Some(1),
                    is_error: true,
                };
            }
            Err(_) => {
                return ShellExecuteResponse {
                    stdout: String::new(),
                    stderr: "Command execution timeout (30s)".to_string(),
                    exit_code: Some(124),
                    is_error: true,
                };
            }
        };

        ShellExecuteResponse {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            is_error: !output.status.success(),
        }
    }
}

// Host function using extism 1.12.0 API
// The host_fn! macro handles JSON serialization/deserialization automatically
// from plugin memory blocks (I64 pointers)
host_fn!(shell_execute(_user_data: (); request: Json<ShellExecuteRequest>) -> Json<ShellExecuteResponse> {
    // Execute command (blocking in host function is acceptable)
    let executor = ShellExecutor::new();
    let response = tokio::runtime::Handle::current()
        .block_on(executor.execute(&request.0.command));
    Ok(Json(response))
});

// Register host function with PluginBuilder (new API pattern)
// This is called during plugin construction, not after
pub fn register_shell_host_functions(builder: PluginBuilder) -> PluginBuilder {
    builder.with_function(
        "shell_execute",
        [ValType::I64],    // Input: memory pointer to JSON request
        [ValType::I64],    // Output: memory pointer to JSON response
        UserData::new(()), // No shared state needed (using unit type)
        shell_execute,     // Function created by host_fn! macro above
    )
}
