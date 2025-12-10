use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;

/// GitHub raw URL for the default system prompt
const SYSTEM_PROMPT_URL: &str = "https://raw.githubusercontent.com/cyrup-ai/kodegen-claude-plugin/refs/heads/main/plugins/kg/SYSTEM_PROMPT.md";

/// Handle the `kodegen claude` subcommand
pub async fn handle_claude(
    toolset: Vec<String>,
    model: String,
    session_id: Option<String>,
    system_prompt: Option<PathBuf>,
    disallowed_tools: String,
    permission_mode: String,
    passthrough_args: Vec<String>,
) -> Result<()> {
    // Auto-configure KODEGEN plugin if not already present
    if super::ensure_plugin_configured() {
        eprintln!("✓ KODEGEN plugin configured for Claude Code");
        eprintln!("  Source: github:cyrup-ai/kodegen-claude-plugin");
    }

    // 1. Find claude binary in PATH
    let claude_bin = find_claude_binary()?;

    // 2. Generate or validate session ID
    let session_id = resolve_session_id(session_id)?;

    // 3. Resolve toolset paths and load tool names
    let tool_names = load_toolsets(&toolset).await?;

    // 4. Load system prompt content (fetch from GitHub or custom file)
    let system_prompt_content = if let Some(path) = system_prompt {
        // User provided custom prompt file - read and use it
        tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read custom system prompt from {:?}", path))?
    } else {
        // Fetch default system prompt from GitHub
        fetch_system_prompt().await?
    };

    // 5. Build allowed-tools list with mcp__plugin_kodegen_kodegen__ prefix
    let allowed_tools: Vec<String> = tool_names
        .iter()
        .map(|tool| format!("mcp__plugin_kg_kodegen__{}", tool))
        .collect();

    // 6. Build command arguments
    let mut cmd = Command::new(&claude_bin);

    // Apply defaults
    cmd.arg("--model").arg(&model);
    cmd.arg("--session-id").arg(&session_id);

    // Apply allowed tools
    if !allowed_tools.is_empty() {
        cmd.arg("--allowed-tools").arg(allowed_tools.join(","));
    }

    // Apply system prompt content (always present - fetched or custom)
    cmd.arg("--system-prompt").arg(&system_prompt_content);

    // Apply disallowed tools
    cmd.arg("--disallowed-tools").arg(&disallowed_tools);

    // Apply permission mode
    cmd.arg("--permission-mode").arg(&permission_mode);

    // Pass through all other arguments
    cmd.args(&passthrough_args);

    // Set stdio to inherit (interactive mode)
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    // 8. Print session ID for user reference
    eprintln!("Session ID: {}", session_id);

    // 9. Spawn and wait for claude process
    let mut child = cmd
        .spawn()
        .context("Failed to spawn claude CLI - ensure 'claude' is installed and in PATH")?;

    let status = child
        .wait()
        .await
        .context("Failed to wait for claude process")?;

    // 10. Print session ID again on exit for easy copying
    eprintln!("\nSession completed: {}", session_id);

    // Exit with same code as claude
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

/// Find the claude binary in PATH
fn find_claude_binary() -> Result<PathBuf> {
    which::which("claude").context(
        "Claude CLI not found in PATH. Install from: https://github.com/anthropics/claude-code",
    )
}

/// Generate new session ID or validate provided one
fn resolve_session_id(provided: Option<String>) -> Result<String> {
    if let Some(id) = provided {
        // Validate it's a valid UUID
        Uuid::parse_str(&id).context("Invalid session ID format - must be a valid UUID")?;
        Ok(id)
    } else {
        // Generate new UUIDv4
        Ok(Uuid::new_v4().to_string())
    }
}

/// Load and merge toolsets from multiple sources
async fn load_toolsets(toolsets: &[String]) -> Result<Vec<String>> {
    crate::cli::load_and_merge_toolsets(toolsets).await
}

/// Fetch the default system prompt from GitHub
async fn fetch_system_prompt() -> Result<String> {
    eprintln!("Fetching system prompt from GitHub...");

    let response = reqwest::get(SYSTEM_PROMPT_URL)
        .await
        .context("Failed to fetch system prompt from GitHub")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch system prompt: HTTP {}", response.status());
    }

    let content = response
        .text()
        .await
        .context("Failed to read system prompt response body")?;

    eprintln!("✓ System prompt fetched ({} bytes)", content.len());

    Ok(content)
}
