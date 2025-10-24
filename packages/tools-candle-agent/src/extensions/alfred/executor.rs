use crate::extensions::alfred::discovery::ScriptType;
use crate::extensions::common::types::{ExtensionError, Result};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

pub async fn execute_script_filter(
    script: &str,
    script_type: &ScriptType,
    query: &str,
    timeout_secs: u64,
) -> Result<String> {
    let interpreter = match script_type {
        ScriptType::Bash => "bash",
        ScriptType::Python => "python3",
        ScriptType::Ruby => "ruby",
        ScriptType::PHP => "php",
        ScriptType::JavaScript => "node",
        ScriptType::AppleScript => "osascript",
        ScriptType::Other(cmd) => cmd.as_str(),
    };

    let child = Command::new(interpreter)
        .arg("-c")
        .arg(script)
        .env("query", query)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let result = timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await;

    match result {
        Ok(Ok(output)) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
        Ok(Err(e)) => Err(ExtensionError::ProcessError(e)),
        Err(_) => Err(ExtensionError::Timeout(timeout_secs)),
    }
}
