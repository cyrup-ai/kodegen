use super::types::{ExtensionError, Result};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

pub struct SandboxConfig {
    pub timeout_secs: u64,
    pub max_memory_mb: Option<u64>,
    pub capture_output: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_memory_mb: Some(512),
            capture_output: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxedOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

pub async fn execute_sandboxed(
    command: &str,
    args: &[String],
    config: SandboxConfig,
) -> Result<SandboxedOutput> {
    let mut cmd = Command::new(command);
    cmd.args(args);

    if config.capture_output {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let child = cmd.spawn()?;

    let result = timeout(
        Duration::from_secs(config.timeout_secs),
        child.wait_with_output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => Ok(SandboxedOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            timed_out: false,
        }),
        Ok(Err(e)) => Err(ExtensionError::ProcessError(e)),
        Err(_) => Ok(SandboxedOutput {
            stdout: String::new(),
            stderr: format!("Process timed out after {} seconds", config.timeout_secs),
            exit_code: None,
            timed_out: true,
        }),
    }
}
