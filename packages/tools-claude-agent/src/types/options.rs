//! Claude Agent options and configuration
//!
//! This module contains the main configuration options for the Claude Agent SDK,
//! including a builder pattern for easy configuration.

use std::collections::HashMap;
use std::path::PathBuf;

use super::agent::{AgentDefinition, SystemPrompt};
use super::hooks::{HookEvent, HookMatcher};
use super::identifiers::{SessionId, ToolName};
use super::mcp::{McpServerConfig, McpServers};
use super::permissions::{CanUseToolCallback, PermissionMode, SettingSource};

// ============================================================================
// Claude Agent Options
// ============================================================================

/// Main options for Claude Agent SDK
#[derive(Clone, Default)]
pub struct ClaudeAgentOptions {
    /// List of tools that Claude is allowed to use
    pub allowed_tools: Vec<ToolName>,
    /// System prompt configuration
    pub system_prompt: Option<SystemPrompt>,
    /// MCP server configurations
    pub mcp_servers: McpServers,
    /// Permission mode for tool execution
    pub permission_mode: Option<PermissionMode>,
    /// Whether to continue from the previous conversation
    pub continue_conversation: bool,
    /// Session ID to resume from
    pub resume: Option<SessionId>,
    /// Maximum number of turns before stopping
    pub max_turns: Option<u32>,
    /// List of tools that Claude is not allowed to use
    pub disallowed_tools: Vec<ToolName>,
    /// AI model to use
    pub model: Option<String>,
    /// Tool name to use for permission prompts
    pub permission_prompt_tool_name: Option<String>,
    /// Working directory for the CLI process
    pub cwd: Option<PathBuf>,
    /// Path to settings file
    pub settings: Option<PathBuf>,
    /// Additional directories to add to the context
    pub add_dirs: Vec<PathBuf>,
    /// Environment variables for the CLI process
    pub env: HashMap<String, String>,
    /// Extra CLI arguments to pass
    pub extra_args: HashMap<String, Option<String>>,
    /// Maximum buffer size for JSON messages (default: 1MB)
    pub max_buffer_size: Option<usize>,
    /// Callback for tool permission checks
    pub can_use_tool: Option<CanUseToolCallback>,
    /// Hook configurations
    pub hooks: Option<HashMap<HookEvent, Vec<HookMatcher>>>,
    /// User identifier
    pub user: Option<String>,
    /// Whether to include partial messages in stream
    pub include_partial_messages: bool,
    /// Whether to fork the session when resuming
    pub fork_session: bool,
    /// Custom agent definitions
    pub agents: Option<HashMap<String, AgentDefinition>>,
    /// Setting sources to load
    pub setting_sources: Option<Vec<SettingSource>>,
}

impl ClaudeAgentOptions {
    /// Create a new builder for `ClaudeAgentOptions`
    #[must_use]
    pub fn builder() -> ClaudeAgentOptionsBuilder {
        ClaudeAgentOptionsBuilder::default()
    }
}

impl std::fmt::Debug for ClaudeAgentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeAgentOptions")
            .field("allowed_tools", &self.allowed_tools)
            .field("system_prompt", &self.system_prompt)
            .field("mcp_servers", &self.mcp_servers)
            .field("permission_mode", &self.permission_mode)
            .field("continue_conversation", &self.continue_conversation)
            .field("resume", &self.resume)
            .field("max_turns", &self.max_turns)
            .field("disallowed_tools", &self.disallowed_tools)
            .field("model", &self.model)
            .field(
                "permission_prompt_tool_name",
                &self.permission_prompt_tool_name,
            )
            .field("cwd", &self.cwd)
            .field("settings", &self.settings)
            .field("add_dirs", &self.add_dirs)
            .field("env", &self.env)
            .field("extra_args", &self.extra_args)
            .field("max_buffer_size", &self.max_buffer_size)
            .field(
                "can_use_tool",
                &self.can_use_tool.as_ref().map(|_| "<callback>"),
            )
            .field(
                "hooks",
                &self
                    .hooks
                    .as_ref()
                    .map(|h| format!("[{} hook types]", h.len())),
            )
            .field("user", &self.user)
            .field("include_partial_messages", &self.include_partial_messages)
            .field("fork_session", &self.fork_session)
            .field("agents", &self.agents)
            .field("setting_sources", &self.setting_sources)
            .finish()
    }
}

// ============================================================================
// Builder for ClaudeAgentOptions
// ============================================================================

/// Builder for `ClaudeAgentOptions`
#[derive(Debug, Default)]
pub struct ClaudeAgentOptionsBuilder {
    options: ClaudeAgentOptions,
}

impl ClaudeAgentOptionsBuilder {
    /// Set allowed tools
    #[must_use]
    pub fn allowed_tools(mut self, tools: Vec<impl Into<ToolName>>) -> Self {
        self.options.allowed_tools = tools.into_iter().map(std::convert::Into::into).collect();
        self
    }

    /// Add an allowed tool
    #[must_use]
    pub fn add_allowed_tool(mut self, tool: impl Into<ToolName>) -> Self {
        self.options.allowed_tools.push(tool.into());
        self
    }

    /// Set system prompt
    #[must_use]
    pub fn system_prompt(mut self, prompt: impl Into<SystemPrompt>) -> Self {
        self.options.system_prompt = Some(prompt.into());
        self
    }

    /// Set MCP servers
    #[must_use]
    pub fn mcp_servers(mut self, servers: HashMap<String, McpServerConfig>) -> Self {
        self.options.mcp_servers = McpServers::Dict(servers);
        self
    }

    /// Set permission mode
    #[must_use]
    pub const fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.options.permission_mode = Some(mode);
        self
    }

    /// Set max turns
    ///
    /// # Panics
    /// Panics if turns exceeds 1000
    #[must_use]
    pub fn max_turns(mut self, turns: u32) -> Self {
        const MAX_ALLOWED_TURNS: u32 = 1000;
        assert!(
            turns <= MAX_ALLOWED_TURNS,
            "max_turns {turns} exceeds maximum allowed: {MAX_ALLOWED_TURNS}"
        );
        self.options.max_turns = Some(turns);
        self
    }

    /// Set working directory
    #[must_use]
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.cwd = Some(path.into());
        self
    }

    /// Set `can_use_tool` callback
    #[must_use]
    pub fn can_use_tool(mut self, callback: CanUseToolCallback) -> Self {
        self.options.can_use_tool = Some(callback);
        self
    }

    /// Set hooks
    #[must_use]
    pub fn hooks(mut self, hooks: HashMap<HookEvent, Vec<HookMatcher>>) -> Self {
        self.options.hooks = Some(hooks);
        self
    }

    /// Build the options
    #[must_use]
    pub fn build(self) -> ClaudeAgentOptions {
        self.options
    }
}
