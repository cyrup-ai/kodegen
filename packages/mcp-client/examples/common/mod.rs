//! Shared utilities for kodegen MCP client examples
//!
//! This module provides common functionality for spawning and connecting to
//! the kodegen server during development/testing.

use anyhow::{Result, Context};
use kodegen_mcp_client::{KodegenConnection, KodegenClient, create_sse_client};
use rmcp::model::{CallToolResult, ServerInfo};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, Mutex as StdMutex, Arc};
use tokio::process::{Child, Command};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;
use serde::de::DeserializeOwned;

/// Default SSE server URL for examples
///
/// Examples automatically spawn an SSE server and connect to this URL.
const DEFAULT_SSE_URL: &str = "http://127.0.0.1:18080/sse";

/// Cached workspace root to avoid repeated cargo metadata executions
static WORKSPACE_ROOT: OnceLock<PathBuf> = OnceLock::new();
static WORKSPACE_ROOT_INIT: StdMutex<()> = StdMutex::new(());

/// Find the workspace root by querying cargo metadata
fn find_workspace_root() -> Result<&'static PathBuf> {
    if let Some(root) = WORKSPACE_ROOT.get() {
        return Ok(root);
    }

    let _lock = WORKSPACE_ROOT_INIT.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {e}"))?;

    if let Some(root) = WORKSPACE_ROOT.get() {
        return Ok(root);
    }

    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .context("Failed to execute cargo metadata")?;

    if !output.status.success() {
        anyhow::bail!("cargo metadata failed (exit code: {:?})", output.status.code());
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Invalid JSON from cargo metadata")?;

    let workspace_root = metadata["workspace_root"]
        .as_str()
        .context("No workspace_root in metadata")?;

    let path = PathBuf::from(workspace_root);
    WORKSPACE_ROOT.set(path).map_err(|_| anyhow::anyhow!("Failed to cache workspace root"))?;
    WORKSPACE_ROOT.get().ok_or_else(|| anyhow::anyhow!("Failed to retrieve cached workspace root"))
}

/// Tool categories supported by the kodegen server
///
/// These map directly to the `--tools` CLI argument and compiled feature flags.
/// Using an enum prevents typos and injection attacks at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum ToolCategory {
    /// File operations (14 tools)
    Filesystem,
    /// Terminal sessions (5 tools)
    Terminal,
    /// Process management (2 tools)
    Process,
    /// Usage tracking (2 tools)
    Introspection,
    /// Prompt management (4 tools)
    Prompt,
    /// Reasoning chains (1 tool)
    SequentialThinking,
    /// Sub-agent delegation (5 tools)
    ClaudeAgent,
    /// Web crawling and search (4 tools)
    Citescrape,
    /// Git operations (20 tools)
    Git,
    /// GitHub API integration (16 tools)
    Github,
    /// Database operations (8 tools)
    Database,
}

/// All available tool categories
///
/// This const ensures all enum variants are constructed, which satisfies clippy's
/// `dead_code` analysis across example binaries that each use only specific categories.
#[allow(dead_code)]
pub const ALL_CATEGORIES: &[ToolCategory] = &[
    ToolCategory::Filesystem,
    ToolCategory::Terminal,
    ToolCategory::Process,
    ToolCategory::Introspection,
    ToolCategory::Prompt,
    ToolCategory::SequentialThinking,
    ToolCategory::ClaudeAgent,
    ToolCategory::Citescrape,
    ToolCategory::Git,
    ToolCategory::Github,
    ToolCategory::Database,
];

impl ToolCategory {
    /// Convert enum to server-expected string format
    ///
    /// These strings MUST match exactly with:
    /// - CLI argument parsing in server/src/cli.rs
    /// - Feature flags in Cargo.toml
    /// - Category checks in server/src/main.rs
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::Terminal => "terminal",
            Self::Process => "process",
            Self::Introspection => "introspection",
            Self::Prompt => "prompt",
            Self::SequentialThinking => "sequential_thinking",
            Self::ClaudeAgent => "claude_agent",
            Self::Citescrape => "citescrape",
            Self::Git => "git",
            Self::Github => "github",
            Self::Database => "database",
        }
    }

    /// Get a slice of all available tool categories
    ///
    /// This is useful for displaying help text or documentation about
    /// all available tool categories across all examples.
    #[allow(dead_code)]
    pub fn all() -> &'static [ToolCategory] {
        ALL_CATEGORIES
    }

    /// Get a display name for this category
    #[allow(dead_code)]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Filesystem => "File operations",
            Self::Terminal => "Terminal sessions",
            Self::Process => "Process management",
            Self::Introspection => "Usage tracking",
            Self::Prompt => "Prompt management",
            Self::SequentialThinking => "Reasoning chains",
            Self::ClaudeAgent => "Sub-agent delegation",
            Self::Citescrape => "Web crawling and search",
            Self::Git => "Git operations",
            Self::Github => "GitHub API integration",
            Self::Database => "Database operations",
        }
    }
}

/// Server process handle for lifecycle management
///
/// Manages the spawned kodegen server process and ensures proper cleanup.
/// The server will be killed when this handle is dropped (safety fallback)
/// or when `shutdown()` is called explicitly (preferred).
#[must_use = "ServerHandle must be kept alive or explicitly shutdown"]
pub struct ServerHandle {
    child: Option<Child>,
}

impl ServerHandle {
    /// Create a new server handle
    fn new(child: Child) -> Self {
        Self {
            child: Some(child),
        }
    }

    /// Gracefully shutdown the server process
    ///
    /// Sends SIGTERM and waits up to 5 seconds for graceful shutdown.
    /// If timeout expires, sends SIGKILL to force termination.
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            eprintln!("🛑 Shutting down SSE server...");

            // Try graceful shutdown first on Unix (SIGTERM)
            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    // Send SIGTERM using kill command
                    let _ = Command::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .status()
                        .await;
                }
            }

            // On Windows, just kill directly since there's no graceful signal
            #[cfg(not(unix))]
            {
                let _ = child.kill().await;
            }

            // Wait up to 5 seconds for graceful shutdown
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                child.wait()
            ).await {
                Ok(Ok(status)) => {
                    eprintln!("✅ Server shut down gracefully (exit code: {})",
                        status.code().unwrap_or(-1));
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
            // Best effort cleanup - use blocking kill since we're in Drop
            eprintln!("⚠️  ServerHandle dropped without explicit shutdown, killing server...");
            let _ = child.start_kill();
        }
    }
}

/// Kill any processes listening on the specified port
///
/// Uses macOS `lsof` to find processes, then kills them with `kill -9`.
/// Ignores errors (idempotent - safe to call even if port is free).
async fn cleanup_port(port: u16) -> Result<()> {
    eprintln!("🧹 Checking for processes on port {port}...");

    // Find PIDs using the port
    let output = Command::new("lsof")
        .args(["-ti", &format!(":{port}")])
        .output()
        .await
        .context("Failed to run lsof - is it installed?")?;

    if output.status.success() && !output.stdout.is_empty() {
        let pids = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids.lines() {
            let pid_str = pid_str.trim();
            if !pid_str.is_empty() {
                eprintln!("   Killing PID {pid_str} on port {port}");
                let _ = Command::new("kill")
                    .args(["-9", pid_str])
                    .status()
                    .await;
            }
        }
    }

    Ok(())
}

/// Connect to SSE server with retry logic
///
/// Polls server readiness until connection succeeds or timeout expires.
/// Uses fixed retry interval (not exponential) since we're waiting for compilation.
async fn connect_with_retry(
    url: &str,
    total_timeout: std::time::Duration,
    retry_interval: std::time::Duration,
) -> Result<(KodegenClient, KodegenConnection)> {
    let start = std::time::Instant::now();
    let mut attempt = 0;
    let mut last_progress_log = start;

    loop {
        attempt += 1;

        // Try to connect
        match create_sse_client(url).await {
            Ok(result) => {
                eprintln!("✅ Connected to SSE server in {:?} (attempt {})", start.elapsed(), attempt);
                return Ok(result);
            }
            Err(e) => {
                let error: anyhow::Error = e.into();

                // Check if timeout expired
                if start.elapsed() >= total_timeout {
                    return Err(error);
                }

                // Log progress every 10 seconds
                if last_progress_log.elapsed() >= std::time::Duration::from_secs(10) {
                    eprintln!("   Still waiting for server... ({:?} elapsed)", start.elapsed());
                    last_progress_log = std::time::Instant::now();
                }

                // Sleep before retry
                tokio::time::sleep(retry_interval).await;
            }
        }
    }
}

/// Connect to kodegen SSE server
///
/// Spawns a new SSE server process and connects to it.
/// Returns both the MCP connection and a server handle for lifecycle management.
///
/// The server process will remain running until `ServerHandle::shutdown()` is called
/// or the handle is dropped. Proper cleanup requires calling `shutdown()` explicitly.
///
/// # Example
///
/// ```no_run
/// use common::ToolCategory;
///
/// let (conn, mut server) = connect_to_server_with_categories(
///     Some(vec![ToolCategory::Filesystem])
/// ).await?;
///
/// // Use connection for MCP operations...
///
/// // Clean up properly
/// conn.close().await?;
/// server.shutdown().await?;
/// ```
///
/// # Errors
///
/// Returns error if the server fails to spawn or connection fails.
pub async fn connect_to_server_with_categories(categories: Option<Vec<ToolCategory>>) -> Result<(KodegenConnection, ServerHandle)> {
    let workspace_root = find_workspace_root()
        .context("Failed to find workspace root")?;

    // Spawn SSE server as child process
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.args(["run", "--package", "kodegen", "--bin", "kodegen", "--", "--sse", "127.0.0.1:18080"]);

    // Pass through GITHUB_TOKEN if set
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        cmd.env("GITHUB_TOKEN", token);
    }

    // Add tool category filter if specified
    if let Some(cats) = categories {
        let cat_strs: Vec<&str> = cats.iter().map(ToolCategory::as_str).collect();
        cmd.arg("--tools").arg(cat_strs.join(","));
    }

    // Clean up any stale servers on the port
    cleanup_port(18080).await.ok(); // Ignore errors - port might be free

    eprintln!("🚀 Starting SSE server...");

    // Spawn the server process and keep the handle
    let child = cmd.spawn()
        .context("Failed to spawn SSE server process")?;

    let server_handle = ServerHandle::new(child);

    // Wait for server to be ready with retry logic
    eprintln!("⏳ Waiting for server to be ready (this may take up to 90s on first compile)...");
    let (_client, connection) = connect_with_retry(
        DEFAULT_SSE_URL,
        std::time::Duration::from_secs(90),   // Total timeout (generous for first compile)
        std::time::Duration::from_millis(500), // Retry interval (fast enough to connect quickly)
    )
    .await
    .context("Failed to connect to SSE server - check server logs")?;

    Ok((connection, server_handle))
}

/// JSONL log entry for tool calls
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

/// Wraps `KodegenClient` to automatically log all tool calls to JSONL
pub struct LoggingClient {
    inner: KodegenClient,
    log_file: Arc<Mutex<BufWriter<tokio::fs::File>>>,
}

impl LoggingClient {
    /// Create a new logging client that writes to the specified log file
    pub async fn new(client: KodegenClient, log_path: impl AsRef<Path>) -> Result<Self> {
        // Create parent directory if needed
        if let Some(parent) = log_path.as_ref().parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create log directory")?;
        }

        // Open file in append mode
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await
            .context("Failed to open log file")?;

        let log_file = Arc::new(Mutex::new(BufWriter::new(file)));

        Ok(Self { inner: client, log_file })
    }

    /// Call a tool and log the request/response
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

    /// Call a tool with typed response and log the request/response
    #[allow(dead_code)]
    pub async fn call_tool_typed<T: DeserializeOwned>(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<T, kodegen_mcp_client::ClientError> {
        // Call raw tool method which logs the full response
        let result = self.call_tool(name, arguments).await?;

        // Extract text content from response
        let text_content = result.content.first()
            .and_then(|c| c.as_text())
            .ok_or_else(|| kodegen_mcp_client::ClientError::ParseError(
                format!("No text content in response from tool '{name}'")
            ))?;

        // Deserialize to target type with context
        serde_json::from_str(&text_content.text)
            .map_err(|e| kodegen_mcp_client::ClientError::ParseError(
                format!("Failed to parse response from tool '{name}': {e}")
            ))
    }

    /// Get server info (passthrough to inner client)
    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.inner.server_info()
    }

    // Private helper to log CallToolResult
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
            Err(e) => LogResult::Error { error: e.to_string() },
        };

        self.log_entry(name, args, log_result, duration).await;
    }

    // Private helper to write log entry
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

    // Private helper to write JSONL
    async fn write_log_entry(&self, entry: &LogEntry) -> Result<()> {
        let json = serde_json::to_string(entry)
            .context("Failed to serialize log entry")?;

        let mut guard = self.log_file.lock().await;
        guard.write_all(json.as_bytes()).await
            .context("Failed to write log entry")?;
        guard.write_all(b"\n").await
            .context("Failed to write newline")?;
        guard.flush().await
            .context("Failed to flush log")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_category_variants_have_string_representation() {
        // This test ensures all ToolCategory variants are constructed
        // and have valid string representations. This satisfies clippy's
        // dead_code analysis across example binaries.
        for category in ToolCategory::all() {
            let s = category.as_str();
            assert!(!s.is_empty(), "Category {:?} has empty string representation", category);
        }
    }

    #[test]
    fn test_all_categories_constant_is_complete() {
        // Verify ALL_CATEGORIES contains all 11 expected categories
        assert_eq!(ALL_CATEGORIES.len(), 11, "Expected 11 tool categories");

        // Verify each expected variant is present
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Filesystem));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Terminal));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Process));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Introspection));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Prompt));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::SequentialThinking));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::ClaudeAgent));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Citescrape));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Git));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Github));
        assert!(ALL_CATEGORIES.contains(&ToolCategory::Database));
    }
}
