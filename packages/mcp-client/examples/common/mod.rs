//! Shared utilities for kodegen MCP client examples
//!
//! This module provides common functionality for spawning and connecting to
//! the kodegen server during development/testing.

use anyhow::{Result, Context};
use kodegen_mcp_client::KodegenClient;
use rmcp::{ServiceExt, transport::TokioChildProcess};
use std::path::PathBuf;
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Timeout for server startup operations
///
/// This prevents indefinite hangs when:
/// - Server has compilation errors
/// - Server deadlocks during initialization
/// - Dependencies are missing
/// - Server panics during startup
const SERVER_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// Tool categories supported by the kodegen server
///
/// These map directly to the `--tools` CLI argument and compiled feature flags.
/// Using an enum prevents typos and injection attacks at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}

/// All available tool categories
///
/// This const ensures all enum variants are constructed, which satisfies clippy's
/// dead_code analysis across example binaries that each use only specific categories.
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
];

impl ToolCategory {
    /// Convert enum to server-expected string format
    ///
    /// These strings MUST match exactly with:
    /// - CLI argument parsing in server/src/cli.rs
    /// - Feature flags in Cargo.toml
    /// - Category checks in server/src/main.rs
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
        }
    }

    /// Get a slice of all available tool categories
    ///
    /// This is useful for displaying help text or documentation about
    /// all available tool categories across all examples.
    pub fn all() -> &'static [ToolCategory] {
        ALL_CATEGORIES
    }

}


/// Find the workspace root by querying cargo metadata
///
/// This is more robust than path manipulation as it works with:
/// - Symlinked packages
/// - Unusual directory layouts
/// - Nested workspaces
///
/// # Errors
///
/// Returns error if cargo metadata fails or workspace root cannot be determined
fn find_workspace_root() -> Result<PathBuf> {
    // Run `cargo metadata` to get workspace root
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    
    let workspace_root = metadata["workspace_root"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No workspace_root in metadata"))?;
    
    Ok(PathBuf::from(workspace_root))
}

/// RAII guard that kills a server process on drop (unless disarmed)
///
/// This prevents process leaks when server connection fails after spawn.
struct ServerProcessGuard {
    pid: Option<u32>,
}

impl ServerProcessGuard {
    /// Create a guard for the given process ID
    fn new(pid: u32) -> Self {
        Self { pid: Some(pid) }
    }
    
    /// Disarm the guard - don't kill process on drop
    /// 
    /// Call this after successful connection to keep server running.
    fn disarm(&mut self) {
        self.pid = None;
    }
}

impl Drop for ServerProcessGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.pid {
            eprintln!("⚠️  Cleaning up zombie server process {}", pid);
            
            // Synchronous kill using sysinfo (same pattern as kill_process tool)
            let mut system = System::new();
            system.refresh_processes(ProcessesToUpdate::All, true);
            
            if let Some(process) = system.process(Pid::from(pid as usize)) {
                if process.kill_with(Signal::Kill).is_some() {
                    eprintln!("✅ Successfully terminated zombie process {}", pid);
                } else {
                    eprintln!("❌ Failed to kill process {}", pid);
                }
            }
            // If process not found, it already terminated - this is fine, no error message needed
        }
    }
}

/// Spawn kodegen server with specific tool categories
///
/// Use the `ToolCategory` enum to specify which tool categories to enable.
/// This provides compile-time validation and prevents injection attacks.
///
/// # Example
///
/// ```no_run
/// use common::ToolCategory;
///
/// let client = connect_to_server_with_categories(
///     Some(vec![ToolCategory::Filesystem, ToolCategory::Terminal])
/// ).await?;
/// ```
///
/// # Errors
///
/// Returns error if the server cannot be started or connection fails.
pub async fn connect_to_server_with_categories(categories: Option<Vec<ToolCategory>>) -> Result<KodegenClient> {
    // Find workspace root using cargo metadata
    let workspace_root = find_workspace_root()
        .context("Failed to find workspace root")?;
    
    // Spawn server with cargo run to always use latest code
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.args(["run", "--package", "kodegen", "--bin", "kodegen", "--"]);
    
    // Add tool category filter if specified
    if let Some(cats) = categories {
        let cat_strs: Vec<&str> = cats.iter().map(|c| c.as_str()).collect();
        cmd.arg("--tools").arg(cat_strs.join(","));
    }
    
    // Show startup progress to user
    eprintln!("🚀 Starting kodegen server (timeout: {}s)...", SERVER_STARTUP_TIMEOUT.as_secs());
    eprintln!("   Command: cargo run --package kodegen --bin kodegen");
    
    let start = std::time::Instant::now();
    
    // Spawn process and get PID BEFORE attempting connection
    let transport = TokioChildProcess::new(cmd)
        .context("Failed to spawn kodegen server process")?;
    
    let pid = transport.id()
        .context("Failed to get child process ID")?;
    
    // Create guard - will kill process on drop unless disarmed
    let mut guard = ServerProcessGuard::new(pid);
    
    // Attempt connection with timeout
    let service_result = timeout(SERVER_STARTUP_TIMEOUT, ().serve(transport)).await;
    
    let service = match service_result {
        Ok(Ok(svc)) => svc,
        Ok(Err(e)) => {
            // Connection failed - guard will kill process on drop
            eprintln!("❌ Server connection failed after {:?}", start.elapsed());
            return Err(e).context("Failed to connect to kodegen server");
        }
        Err(_) => {
            // Timeout - guard will kill process on drop
            eprintln!("❌ Server startup timed out after {:?}", start.elapsed());
            anyhow::bail!(
                "Server failed to start within {}s. Possible causes:\n\
                 - Compilation errors in server code\n\
                 - Server deadlock during initialization\n\
                 - Missing dependencies\n\
                 - Check server logs for details\n\
                 - Run manually: cargo run --package kodegen --bin kodegen",
                SERVER_STARTUP_TIMEOUT.as_secs()
            );
        }
    };
    
    // Success! Don't kill the server
    guard.disarm();
    
    eprintln!("✅ Server connected in {:?}", start.elapsed());
    Ok(KodegenClient::from_service(service))
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
        // Verify ALL_CATEGORIES contains all 10 expected categories
        assert_eq!(ALL_CATEGORIES.len(), 10, "Expected 10 tool categories");

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
    }
}
