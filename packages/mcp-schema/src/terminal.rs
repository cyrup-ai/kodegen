//! Terminal tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// START TERMINAL COMMAND
// ============================================================================

fn default_initial_delay() -> u64 {
    100
}

/// Arguments for `start_terminal_command` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartTerminalCommandArgs {
    /// The shell command to execute
    pub command: String,

    /// Initial delay in milliseconds before returning first response (default: 100ms)
    /// Allows quick commands (pwd, echo) to complete before returning
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// Shell to use (optional, defaults to system shell)
    #[serde(default)]
    pub shell: Option<String>,
}

/// Prompt arguments for `start_terminal_command` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartTerminalCommandPromptArgs {}

// ============================================================================
// READ TERMINAL OUTPUT
// ============================================================================

fn default_length() -> usize {
    100
}

/// Arguments for `read_terminal_output` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadTerminalOutputArgs {
    /// Process ID to read output from
    pub pid: u32,

    /// Offset for pagination (0 = start, negative = tail from end)
    #[serde(default)]
    pub offset: i64,

    /// Maximum lines to return (default: 100)
    #[serde(default = "default_length")]
    pub length: usize,
}

/// Prompt arguments for `read_terminal_output` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadTerminalOutputPromptArgs {}

// ============================================================================
// SEND TERMINAL INPUT
// ============================================================================

fn default_append_newline() -> bool {
    true
}

/// Arguments for `send_terminal_input` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendTerminalInputArgs {
    /// Process ID to send input to
    pub pid: u32,

    /// Input text to send
    pub input: String,

    /// Append newline to execute command (default: true)
    /// Set to false for raw input like Ctrl+C or partial commands
    #[serde(default = "default_append_newline")]
    pub append_newline: bool,
}

/// Prompt arguments for `send_terminal_input` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SendTerminalInputPromptArgs {}

// ============================================================================
// STOP TERMINAL COMMAND
// ============================================================================

/// Arguments for `stop_terminal_command` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StopTerminalCommandArgs {
    /// Process ID to terminate
    pub pid: u32,
}

/// Prompt arguments for `stop_terminal_command` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StopTerminalCommandPromptArgs {}

// ============================================================================
// LIST TERMINAL COMMANDS
// ============================================================================

/// Arguments for `list_terminal_commands` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTerminalCommandsArgs {}

/// Prompt arguments for `list_terminal_commands` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTerminalCommandsPromptArgs {}
