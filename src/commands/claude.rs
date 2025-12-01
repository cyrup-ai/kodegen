use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;
use crate::embedded;

/// Handle the `kodegen claude` subcommand
pub async fn handle_claude(
    toolset: Vec<String>,
    model: String,
    session_id: Option<String>,
    system_prompt: Option<PathBuf>,
    disallowed_tools: Option<Vec<String>>,
    mcp_config: Option<Vec<String>>,
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

    // 4. Load system prompt content (embedded default or custom file)
    let system_prompt_content = if let Some(path) = system_prompt {
        // User provided custom prompt file - read and use it
        tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read custom system prompt from {:?}", path))?
    } else {
        // Use embedded default from bundled .kodegen/claude/SYSTEM_PROMPT.md
        embedded::system_prompt()
            .context("Embedded SYSTEM_PROMPT.md not found - this is a build error")?
            .to_string()
    };

    // 5. Build allowed-tools list with mcp__plugin_kodegen_kodegen__ prefix
    let allowed_tools: Vec<String> = tool_names
        .iter()
        .map(|tool| format!("mcp__plugin_kodegen_kodegen__{}", tool))
        .collect();

    // 6. Build disallowed-tools list (defaults if not provided)
    let disallowed = disallowed_tools.unwrap_or_else(|| {
        vec![
            "Bash".to_string(),
            "Read".to_string(),
            "Write".to_string(),
            "Edit".to_string(),
            "Update".to_string(),
            "WebSearch".to_string(),
            "Fetch".to_string(),
        ]
    });

    // 7. Build MCP config dynamically if not provided
    let mcp_config_json = if let Some(configs) = mcp_config {
        configs
    } else {
        vec![build_kodegen_mcp_config(&toolset).await?]
    };

    // 8. Build command arguments
    let mut cmd = Command::new(&claude_bin);

    // Apply defaults
    cmd.arg("--model").arg(&model);
    cmd.arg("--session-id").arg(&session_id);

    // Apply disallowed tools
    if !disallowed.is_empty() {
        cmd.arg("--disallowed-tools").arg(disallowed.join(","));
    }

    // Apply allowed tools
    if !allowed_tools.is_empty() {
        cmd.arg("--allowed-tools").arg(allowed_tools.join(","));
    }

    // Apply system prompt content (always present - embedded or custom)
    cmd.arg("--system-prompt").arg(&system_prompt_content);

    // Apply MCP configs
    for config in &mcp_config_json {
        cmd.arg("--mcp-config").arg(config);
    }

    // Pass through all other arguments
    cmd.args(&passthrough_args);

    // Set stdio to inherit (interactive mode)
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    // 9. Print session ID for user reference
    eprintln!("Session ID: {}", session_id);

    // 10. Spawn and wait for claude process
    let mut child = cmd.spawn()
        .context("Failed to spawn claude CLI - ensure 'claude' is installed and in PATH")?;

    let status = child.wait().await
        .context("Failed to wait for claude process")?;

    // 11. Print session ID again on exit for easy copying
    eprintln!("\nSession completed: {}", session_id);

    // Exit with same code as claude
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

/// Find the claude binary in PATH
fn find_claude_binary() -> Result<PathBuf> {
    which::which("claude")
        .context("Claude CLI not found in PATH. Install from: https://github.com/anthropics/claude-code")
}

/// Generate new session ID or validate provided one
fn resolve_session_id(provided: Option<String>) -> Result<String> {
    if let Some(id) = provided {
        // Validate it's a valid UUID
        Uuid::parse_str(&id)
            .context("Invalid session ID format - must be a valid UUID")?;
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

/// Build MCP config JSON for kodegen stdio server
/// Mirrors: claude mcp add --transport stdio kodegen -- kodegen --no-tls --toolset core
async fn build_kodegen_mcp_config(toolsets: &[String]) -> Result<String> {
    // Check if we should use TLS by attempting to connect to the kodegen server
    let use_tls = should_use_tls().await;

    // Build the command args
    let mut cmd_args = vec!["kodegen".to_string()];

    if !use_tls {
        cmd_args.push("--no-tls".to_string());
    }

    // Add toolset args
    for toolset in toolsets {
        cmd_args.push("--toolset".to_string());
        cmd_args.push(toolset.clone());
    }

    // Build MCP config JSON
    let config = serde_json::json!({
        "mcpServers": {
            "kodegen": {
                "command": "kodegen",
                "args": cmd_args[1..].to_vec(), // Skip the "kodegen" binary name
                "transport": "stdio"
            }
        }
    });

    Ok(config.to_string())
}

/// Determine if we should use TLS by checking if kodegen servers are running with TLS
async fn should_use_tls() -> bool {
    // Try HTTPS first on port 30438 (filesystem server)
    let https_url = "https://mcp.kodegen.ai:30438/mcp/health";
    if reqwest::get(https_url).await.is_ok() {
        return true;
    }

    // If HTTPS fails, assume no TLS
    false
}
