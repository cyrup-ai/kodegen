//! CLI command building logic for subprocess transport

use std::collections::HashMap;
use tokio::process::Command;

use crate::types::agent::SystemPrompt;
use crate::types::mcp::{McpServerConfig, McpServers};
use crate::types::options::ClaudeAgentOptions;
use crate::types::permissions::{PermissionMode, SettingSource};

use super::config::{ALLOWED_EXTRA_FLAGS, PromptInput};

/// Command builder for Claude CLI
pub struct CommandBuilder<'a> {
    cli_path: &'a std::path::Path,
    prompt: &'a PromptInput,
    options: &'a ClaudeAgentOptions,
}

impl<'a> CommandBuilder<'a> {
    /// Create a new command builder
    pub fn new(
        cli_path: &'a std::path::Path,
        prompt: &'a PromptInput,
        options: &'a ClaudeAgentOptions,
    ) -> Self {
        Self {
            cli_path,
            prompt,
            options,
        }
    }

    /// Build the complete CLI command with all arguments
    pub fn build(&self) -> Command {
        let mut cmd = Command::new(self.cli_path);

        // Base arguments
        cmd.arg("--print")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose");

        // System prompt
        if let Some(ref system_prompt) = self.options.system_prompt {
            match system_prompt {
                SystemPrompt::String(s) => {
                    cmd.arg("--system-prompt").arg(s);
                }
                SystemPrompt::Preset(preset) => {
                    if let Some(ref append) = preset.append {
                        cmd.arg("--append-system-prompt").arg(append);
                    }
                }
            }
        }

        self.add_tool_args(&mut cmd);
        self.add_configuration_args(&mut cmd);
        self.add_session_args(&mut cmd);
        self.add_mcp_args(&mut cmd);
        self.add_extra_args(&mut cmd);

        // Prompt handling based on mode
        match self.prompt {
            PromptInput::Stream => {
                cmd.arg("--input-format").arg("stream-json");
            }
            PromptInput::String(s) => {
                cmd.arg("--").arg(s);
            }
        }

        cmd
    }

    /// Add tool-related arguments
    fn add_tool_args(&self, cmd: &mut Command) {
        if !self.options.allowed_tools.is_empty() {
            let tools: Vec<String> = self
                .options
                .allowed_tools
                .iter()
                .map(|t| t.as_str().to_string())
                .collect();
            cmd.arg("--allowedTools").arg(tools.join(","));
        }

        if !self.options.disallowed_tools.is_empty() {
            let tools: Vec<String> = self
                .options
                .disallowed_tools
                .iter()
                .map(|t| t.as_str().to_string())
                .collect();
            cmd.arg("--disallowedTools").arg(tools.join(","));
        }
    }

    /// Add configuration arguments (model, max turns, permissions)
    fn add_configuration_args(&self, cmd: &mut Command) {
        if let Some(max_turns) = self.options.max_turns {
            cmd.arg("--max-turns").arg(max_turns.to_string());
        }

        if let Some(ref model) = self.options.model {
            cmd.arg("--model").arg(model);
        }

        if let Some(ref tool) = self.options.permission_prompt_tool_name {
            cmd.arg("--permission-prompt-tool").arg(tool);
        }

        if let Some(ref mode) = self.options.permission_mode {
            let mode_str = match mode {
                PermissionMode::Default => "default",
                PermissionMode::AcceptEdits => "acceptEdits",
                PermissionMode::Plan => "plan",
                PermissionMode::BypassPermissions => "bypassPermissions",
            };
            cmd.arg("--permission-mode").arg(mode_str);
        }
    }

    /// Add session-related arguments
    fn add_session_args(&self, cmd: &mut Command) {
        if self.options.continue_conversation {
            cmd.arg("--continue");
        }

        if let Some(ref session_id) = self.options.resume {
            cmd.arg("--resume").arg(session_id.as_str());
        }

        if let Some(ref settings) = self.options.settings {
            cmd.arg("--settings").arg(settings);
        }

        for dir in &self.options.add_dirs {
            cmd.arg("--add-dir").arg(dir);
        }

        if self.options.include_partial_messages {
            cmd.arg("--include-partial-messages");
        }

        if self.options.fork_session {
            cmd.arg("--fork-session");
        }

        if let Some(ref agents) = self.options.agents {
            let agents_json = serde_json::to_string(agents).unwrap_or_default();
            cmd.arg("--agents").arg(agents_json);
        }
    }

    /// Add MCP server configuration
    fn add_mcp_args(&self, cmd: &mut Command) {
        match &self.options.mcp_servers {
            McpServers::Dict(servers) => {
                if !servers.is_empty() {
                    let mut config_map = HashMap::new();
                    for (name, config) in servers {
                        config_map.insert(name.clone(), serialize_mcp_config(config));
                    }
                    let config_json = serde_json::json!({
                        "mcpServers": config_map
                    });
                    cmd.arg("--mcp-config").arg(config_json.to_string());
                }
            }
            McpServers::Path(path) => {
                cmd.arg("--mcp-config").arg(path);
            }
            McpServers::None => {}
        }
    }

    /// Add setting sources and extra arguments
    fn add_extra_args(&self, cmd: &mut Command) {
        if let Some(ref sources) = self.options.setting_sources {
            let sources_str: Vec<&str> = sources
                .iter()
                .map(|s| match s {
                    SettingSource::User => "user",
                    SettingSource::Project => "project",
                    SettingSource::Local => "local",
                })
                .collect();
            cmd.arg("--setting-sources").arg(sources_str.join(","));
        } else {
            cmd.arg("--setting-sources").arg("");
        }

        for (flag, value) in &self.options.extra_args {
            if ALLOWED_EXTRA_FLAGS.contains(&flag.as_str()) {
                if let Some(v) = value {
                    cmd.arg(format!("--{flag}")).arg(v);
                } else {
                    cmd.arg(format!("--{flag}"));
                }
            }
        }
    }
}

/// Serialize MCP config for CLI
fn serialize_mcp_config(config: &McpServerConfig) -> serde_json::Value {
    match config {
        McpServerConfig::Stdio(stdio) => {
            let mut obj = serde_json::json!({
                "command": stdio.command,
            });
            if let Some(ref args) = stdio.args {
                obj["args"] = serde_json::json!(args);
            }
            if let Some(ref env) = stdio.env {
                obj["env"] = serde_json::json!(env);
            }
            if let Some(ref server_type) = stdio.server_type {
                obj["type"] = serde_json::json!(server_type);
            }
            obj
        }
        McpServerConfig::Sse(sse) => {
            serde_json::json!({
                "type": sse.server_type,
                "url": sse.url,
                "headers": sse.headers,
            })
        }
        McpServerConfig::Http(http) => {
            serde_json::json!({
                "type": http.server_type,
                "url": http.url,
                "headers": http.headers,
            })
        }
        McpServerConfig::Sdk(sdk) => {
            serde_json::json!({
                "type": "sdk",
                "name": sdk.name,
            })
        }
    }
}
