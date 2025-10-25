//! Shared utilities for browser SSE server examples
//!
//! This module spawns the local kodegen-browser SSE server and connects to it.

use anyhow::{Context, Result};
use kodegen_mcp_client::{KodegenClient, KodegenConnection, create_sse_client};
use rmcp::model::{CallToolResult, ServerInfo};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::sync::{Mutex as StdMutex, OnceLock};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::sync::Arc;

/// Browser SSE server configuration
const SSE_PORT: u16 = 30447;
const BINARY_NAME: &str = "kodegen-process";
const PACKAGE_NAME: &str = "kodegen_tools_process";

/// SSE server URL for browser examples
const SSE_URL: &str = "http://127.0.0.1:30447/sse";

/// Cached workspace root
static WORKSPACE_ROOT: OnceLock<PathBuf> = OnceLock::new();
static WORKSPACE_ROOT_INIT: StdMutex<()> = StdMutex::new(());

/// Find workspace root using cargo metadata
pub fn find_workspace_root() -> Result<&'static PathBuf> {
    if let Some(root) = WORKSPACE_ROOT.get() {
        return Ok(root);
    }

    let _lock = WORKSPACE_ROOT_INIT
        .lock()
        .map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

    if let Some(root) = WORKSPACE_ROOT.get() {
        return Ok(root);
    }

    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .context("Failed to execute cargo metadata")?;

    if !output.status.success() {
        anyhow::bail!(
            "cargo metadata failed (exit code: {:?})",
            output.status.code()
        );
    }

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Invalid JSON from cargo metadata")?;

    let workspace_root = metadata["workspace_root"]
        .as_str()
        .context("No workspace_root in metadata")?;

    let path = PathBuf::from(workspace_root);
    WORKSPACE_ROOT
        .set(path)
        .map_err(|_| anyhow::anyhow!("Failed to cache workspace root"))?;
    WORKSPACE_ROOT
        .get()
        .ok_or_else(|| anyhow::anyhow!("Failed to retrieve cached workspace root"))
}

/// Server process handle
#[must_use = "ServerHandle must be kept alive or explicitly shutdown"]
pub struct ServerHandle {
    child: Option<Child>,
}

impl ServerHandle {
    pub fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            eprintln!("🛑 Shutting down SSE server...");

            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    let _ = Command::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .status()
                        .await;
                }
            }

            #[cfg(not(unix))]
            {
                let _ = child.kill().await;
            }

            match tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await {
                Ok(Ok(status)) => {
                    eprintln!(
                        "✅ Server shut down gracefully (exit code: {})",
                        status.code().unwrap_or(-1)
                    );
                }
                Ok(Err(e)) => {
                    eprintln!("⚠️  Error waiting for server: {e}");
                    let _ = child.kill().await;
                }
                Err(_) => {
                    eprintln!("⚠️  Server shutdown timeout, killing forcefully...");
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
        }
        Ok(())
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            eprintln!("⚠️  ServerHandle dropped without explicit shutdown, killing server...");
            let _ = child.start_kill();
        }
    }
}

/// Kill processes on specified port
pub async fn cleanup_port(port: u16) -> Result<()> {
    eprintln!("🧹 Checking for processes on port {port}...");

    let output = Command::new("lsof")
        .args(["-ti", &format!(":{port}")])
        .output()
        .await
        .context("Failed to run lsof")?;

    if output.status.success() && !output.stdout.is_empty() {
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids.lines() {
            let pid_str = pid_str.trim();
            if !pid_str.is_empty() {
                eprintln!("   Killing PID {pid_str} on port {port}");
                let _ = Command::new("kill").args(["-9", pid_str]).status().await;
            }
        }
    }

    Ok(())
}

/// Connect to SSE server with retry
pub async fn connect_with_retry(
    url: &str,
    total_timeout: std::time::Duration,
    retry_interval: std::time::Duration,
) -> Result<(KodegenClient, KodegenConnection)> {
    let start = std::time::Instant::now();
    let mut attempt = 0;
    let mut last_progress_log = start;

    loop {
        attempt += 1;

        match create_sse_client(url).await {
            Ok(result) => {
                eprintln!(
                    "✅ Connected to SSE server in {:?} (attempt {})",
                    start.elapsed(),
                    attempt
                );
                return Ok(result);
            }
            Err(e) => {
                let error: anyhow::Error = e.into();

                if start.elapsed() >= total_timeout {
                    return Err(error);
                }

                if last_progress_log.elapsed() >= std::time::Duration::from_secs(10) {
                    eprintln!(
                        "   Still waiting for server... ({:?} elapsed)",
                        start.elapsed()
                    );
                    last_progress_log = std::time::Instant::now();
                }

                tokio::time::sleep(retry_interval).await;
            }
        }
    }
}

/// Connect to local browser SSE server
pub async fn connect_to_local_sse_server() -> Result<(KodegenConnection, ServerHandle)> {
    let workspace_root = find_workspace_root().context("Failed to find workspace root")?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.args([
        "run",
        "--package",
        PACKAGE_NAME,
        "--bin",
        BINARY_NAME,
        "--",
        "--sse",
        &format!("127.0.0.1:{}", SSE_PORT),
    ]);

    // Pass through GITHUB_TOKEN if set
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        cmd.env("GITHUB_TOKEN", token);
    }

    cleanup_port(SSE_PORT).await.ok();

    eprintln!("🚀 Starting kodegen-process SSE server on port 30447...", BINARY_NAME, SSE_PORT);

    let child = cmd.spawn().context("Failed to spawn SSE server process")?;
    let server_handle = ServerHandle::new(child);

    eprintln!("⏳ Waiting for server to be ready (this may take up to 90s on first compile)...");
    let (_client, connection) = connect_with_retry(
        SSE_URL,
        std::time::Duration::from_secs(90),
        std::time::Duration::from_millis(500),
    )
    .await
    .context("Failed to connect to SSE server")?;

    Ok((connection, server_handle))
}

/// JSONL log entry
#[derive(Debug, serde::Serialize)]
pub struct LogEntry {
    timestamp: String,
    tool: String,
    args: serde_json::Value,
    duration_ms: u64,
    #[serde(flatten)]
    result: LogResult,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum LogResult {
    Success { response: serde_json::Value },
    Error { error: String },
}

/// Logging wrapper for KodegenClient
pub struct LoggingClient {
    inner: KodegenClient,
    log_file: Arc<Mutex<BufWriter<tokio::fs::File>>>,
}

impl LoggingClient {
    pub async fn new(client: KodegenClient, log_path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = log_path.as_ref().parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create log directory")?;
        }

        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await
            .context("Failed to open log file")?;

        let log_file = Arc::new(Mutex::new(BufWriter::new(file)));

        Ok(Self {
            inner: client,
            log_file,
        })
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, kodegen_mcp_client::ClientError> {
        let start = tokio::time::Instant::now();
        let result = self.inner.call_tool(name, arguments.clone()).await;
        let duration = start.elapsed();

        self.log_call(name, arguments, &result, duration).await;
        result
    }

    pub async fn call_tool_typed<T: DeserializeOwned>(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<T, kodegen_mcp_client::ClientError> {
        let result = self.call_tool(name, arguments).await?;

        let text_content = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .ok_or_else(|| {
                kodegen_mcp_client::ClientError::ParseError(format!(
                    "No text content in response from tool '{name}'"
                ))
            })?;

        serde_json::from_str(&text_content.text).map_err(|e| {
            kodegen_mcp_client::ClientError::ParseError(format!(
                "Failed to parse response from tool '{name}': {e}"
            ))
        })
    }

    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.inner.server_info()
    }

    async fn log_call(
        &self,
        name: &str,
        args: serde_json::Value,
        result: &Result<CallToolResult, kodegen_mcp_client::ClientError>,
        duration: std::time::Duration,
    ) {
        let log_result = match result {
            Ok(r) => {
                let response = serde_json::to_value(r)
                    .unwrap_or_else(|_| serde_json::json!({"serialization_error": true}));
                LogResult::Success { response }
            }
            Err(e) => LogResult::Error {
                error: e.to_string(),
            },
        };

        self.log_entry(name, args, log_result, duration).await;
    }

    async fn log_entry(
        &self,
        name: &str,
        args: serde_json::Value,
        result: LogResult,
        duration: std::time::Duration,
    ) {
        let entry = LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            tool: name.to_string(),
            args,
            duration_ms: duration.as_millis() as u64,
            result,
        };

        if let Err(e) = self.write_log_entry(&entry).await {
            eprintln!("⚠️  Failed to write log entry: {e}");
        }
    }

    async fn write_log_entry(&self, entry: &LogEntry) -> Result<()> {
        let json = serde_json::to_string(entry).context("Failed to serialize log entry")?;

        let mut guard = self.log_file.lock().await;
        guard
            .write_all(json.as_bytes())
            .await
            .context("Failed to write log entry")?;
        guard
            .write_all(b"\n")
            .await
            .context("Failed to write newline")?;
        guard.flush().await.context("Failed to flush log")?;

        Ok(())
    }
}
