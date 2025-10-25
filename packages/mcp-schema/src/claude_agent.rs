//! Claude Agent tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// PROMPT INPUT TYPES (shared across tools)
// ============================================================================

/// Input for agent prompts - can be plain string or template
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "value")]
pub enum PromptInput {
    /// Plain text prompt
    #[serde(rename = "string")]
    String(String),

    /// Template-based prompt with parameters
    #[serde(rename = "template")]
    Template(PromptTemplateInput),
}

/// Template reference with parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptTemplateInput {
    /// Template name (e.g., "code_review", "bug_fix")
    pub name: String,

    /// Parameters to pass to template rendering
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
}

// ============================================================================
// SPAWN CLAUDE AGENT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpawnClaudeAgentArgs {
    /// Instructions and context for the spawned Claude agent. Describes the task to be performed autonomously. Can be a plain string or template with parameters.
    pub prompt: PromptInput,

    /// Number of identical agents to spawn (default: 1)
    #[serde(default = "default_worker_count")]
    pub worker_count: u32,

    /// System prompt to define agent behavior
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Tools the agent CAN use (allowlist)
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Tools the agent CANNOT use (blocklist)
    #[serde(default)]
    pub disallowed_tools: Vec<String>,

    /// Max conversation turns (default: 10)
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,

    /// AI model to use
    #[serde(default)]
    pub model: Option<String>,

    /// Working directory for agent operations
    #[serde(default)]
    pub cwd: Option<String>,

    /// Additional context directories
    #[serde(default)]
    pub add_dirs: Vec<String>,

    /// Initial delay before returning (ms, default: 500)
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// Session label prefix (appends -1, -2, etc.)
    #[serde(default)]
    pub label: Option<String>,
}

fn default_worker_count() -> u32 {
    1
}
fn default_max_turns() -> u32 {
    10
}
fn default_initial_delay() -> u64 {
    500
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpawnClaudeAgentPromptArgs {}

// ============================================================================
// SEND CLAUDE AGENT PROMPT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendClaudeAgentPromptArgs {
    /// Session ID to send prompt to
    pub session_id: String,

    /// Prompt to send (continues conversation) - can be plain string or template
    pub prompt: PromptInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendClaudeAgentPromptPromptArgs {}

// ============================================================================
// READ CLAUDE AGENT OUTPUT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadClaudeAgentOutputArgs {
    /// Session ID to read from
    pub session_id: String,

    /// Offset for pagination (0=start, negative=tail from end)
    #[serde(default)]
    pub offset: i64,

    /// Max messages to return (default: 50)
    #[serde(default = "default_length")]
    pub length: usize,
}

fn default_length() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadClaudeAgentOutputPromptArgs {}

// ============================================================================
// LIST CLAUDE AGENTS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListClaudeAgentsArgs {
    /// Include completed sessions (default: true)
    #[serde(default = "default_true")]
    pub include_completed: bool,

    /// Lines of last output per agent (default: 3)
    #[serde(default = "default_last_output_lines")]
    pub last_output_lines: usize,
}

fn default_true() -> bool {
    true
}
fn default_last_output_lines() -> usize {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListClaudeAgentsPromptArgs {}

// ============================================================================
// TERMINATE CLAUDE AGENT SESSION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TerminateClaudeAgentSessionArgs {
    /// Session ID to terminate
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TerminateClaudeAgentSessionPromptArgs {}

// ============================================================================
// MEMORY TOOLS (for candle-agent)
// ============================================================================

// ========== Memorize Tool ==========

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MemorizeArgs {
    /// Library name to store the memory in
    pub library: String,
    /// Content to memorize
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MemorizePromptArgs {}

// ========== Recall Tool ==========

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecallArgs {
    /// Library name to search in
    pub library: String,
    /// Context/query to search for
    pub context: String,
    /// Maximum number of results (default: 10)
    #[serde(default = "default_recall_limit")]
    pub limit: usize,
}

fn default_recall_limit() -> usize {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecallPromptArgs {}

// ========== List Memory Libraries Tool ==========

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListMemoryLibrariesArgs {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListMemoryLibrariesPromptArgs {}
