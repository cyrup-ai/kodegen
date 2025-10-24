use crate::extensions::common::types::{ExtensionError, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

pub async fn execute_script_command(
    script_path: &Path,
    args: &[String],
    timeout_secs: u64,
) -> Result<CommandOutput> {
    let child = Command::new(script_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let result = timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await;

    match result {
        Ok(Ok(output)) => Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        }),
        Ok(Err(e)) => Err(ExtensionError::ProcessError(e)),
        Err(_) => Err(ExtensionError::Timeout(timeout_secs)),
    }
}
