//! Shared utilities for kodegen MCP client examples
//!
//! This module provides common functionality for spawning and connecting to
//! the kodegen server during development/testing.

use anyhow::{Result, Context};
use kodegen_mcp_client::KodegenClient;
use rmcp::{
    ServiceExt, 
    transport::TokioChildProcess,
    model::{ClientCapabilities, ClientInfo, Implementation},
};
use std::path::PathBuf;
use std::sync::{OnceLock, Mutex};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Calculate adaptive timeout for server startup operations
///
/// This prevents indefinite hangs when:
/// - Server has compilation errors
/// - Server deadlocks during initialization
/// - Dependencies are missing
/// - Server panics during startup
///
/// Timeout adapts based on:
/// - KODEGEN_SERVER_TIMEOUT env var (if set)
/// - Debug vs release build (debug gets 3x multiplier)
///
/// # Returns
/// - Custom timeout from env var if KODEGEN_SERVER_TIMEOUT is set and parseable
/// - 45s for debug builds (15s base × 3)
/// - 15s for release builds (15s base × 1)
fn server_startup_timeout() -> Duration {
    if let Ok(timeout) = std::env::var("KODEGEN_SERVER_TIMEOUT")
        && let Ok(secs) = timeout.parse::<u64>() {
            return Duration::from_secs(secs);
        }

    let base = Duration::from_secs(15);
    let multiplier = if cfg!(debug_assertions) { 3 } else { 1 };

    base * multiplier
}

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

    /// Get a display name for this category
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
        }
    }
}

/// Print available tool categories to stderr
///
/// This helper is called during server connection to show what categories
/// are available. Using it ensures all ToolCategory variants are constructed.
pub fn print_available_categories() {
    eprintln!("📦 Available tool categories:");
    for category in ToolCategory::all() {
        eprintln!("   - {} ({})", category.display_name(), category.as_str());
    }
}

/// Extract text content from MCP CallToolResult
///
/// Cached workspace root to avoid repeated cargo metadata executions
///
/// This is populated on first call to find_workspace_root() and reused for all
/// subsequent calls. Cargo metadata takes 50-100ms per execution, so caching
/// provides significant performance improvements in test suites.
///
/// Uses OnceLock<PathBuf> with a separate Mutex<()> guard instead of 
/// OnceLock::get_or_try_init because the latter requires unstable features.
/// The mutex is only held briefly during initialization, so contention is negligible.
static WORKSPACE_ROOT: OnceLock<PathBuf> = OnceLock::new();
static WORKSPACE_ROOT_INIT: Mutex<()> = Mutex::new(());

/// Find the workspace root by querying cargo metadata
///
/// This function is cached - the first call executes `cargo metadata`,
/// subsequent calls return the cached result.
///
/// # Performance
/// - First call: ~50-100ms (cargo metadata execution)
/// - Subsequent calls: <1µs (cache lookup)
///
/// # Safety
/// This function uses compile-time constants and is safe from injection:
/// - `env!("CARGO_MANIFEST_DIR")` is set by rustc at compile time
/// - No user input is passed to the shell command
/// - Error messages do not leak sensitive information
///
/// # Errors
/// Returns error if:
/// - cargo is not in PATH
/// - cargo metadata fails (corrupted Cargo.toml, etc.)
/// - workspace_root field is missing from metadata
fn find_workspace_root() -> Result<&'static PathBuf> {
    // Fast path: already initialized
    if let Some(root) = WORKSPACE_ROOT.get() {
        return Ok(root);
    }

    // Slow path: need to initialize
    let _lock = WORKSPACE_ROOT_INIT.lock().map_err(|e| anyhow::anyhow!("Lock poisoned: {}", e))?;

    // Check again in case another thread initialized while we were waiting
    if let Some(root) = WORKSPACE_ROOT.get() {
        return Ok(root);
    }

    // Actually initialize
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
    
    // Store in OnceLock - this should always succeed since we hold the lock
    WORKSPACE_ROOT.set(path).map_err(|_| anyhow::anyhow!("Failed to cache workspace root"))?;
    
    // Return the cached value - we just set it, so this should always succeed
    WORKSPACE_ROOT.get().ok_or_else(|| anyhow::anyhow!("Failed to retrieve cached workspace root"))
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

    // Ensure child process is killed if connection fails
    cmd.kill_on_drop(true);

    let timeout_duration = server_startup_timeout();

    // Show startup progress to user
    eprintln!("🚀 Starting kodegen server (timeout: {}s)...", timeout_duration.as_secs());
    eprintln!("   Command: cargo run --package kodegen --bin kodegen");

    // Print available categories if KODEGEN_SHOW_CATEGORIES env var is set
    if std::env::var("KODEGEN_SHOW_CATEGORIES").is_ok() {
        print_available_categories();
    }
    
    let start = std::time::Instant::now();
    
    // Spawn process - will be killed on drop if connection fails (due to kill_on_drop above)
    let transport = TokioChildProcess::new(cmd)
        .context("Failed to spawn kodegen server process")?;

    // Create client info for MCP initialization
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "kodegen-example-client".to_string(),
            title: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            website_url: None,
            icons: None,
        },
    };

    // Attempt connection with timeout - transport consumed here
    match timeout(timeout_duration, client_info.serve(transport)).await {
        Ok(Ok(service)) => {
            // Success! Transport ownership transferred to service
            eprintln!("✅ Server connected in {:?}", start.elapsed());
            Ok(KodegenClient::from_service(service))
        }
        Ok(Err(e)) => {
            // Connection failed - transport dropped, child killed automatically
            eprintln!("❌ Server connection failed after {:?}", start.elapsed());
            Err(e).context("Failed to connect to kodegen server")
        }
        Err(_) => {
            // Timeout - transport dropped, child killed automatically
            eprintln!("❌ Server startup timed out after {:?}", start.elapsed());
            anyhow::bail!(
                "Server failed to start within {}s. Possible causes:\n\
                 - Compilation errors in server code\n\
                 - Server deadlock during initialization\n\
                 - Missing dependencies\n\
                 - Check server logs for details\n\
                 - Run manually: cargo run --package kodegen --bin kodegen",
                timeout_duration.as_secs()
            );
        }
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
