//! Process tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// LIST PROCESSES
// ============================================================================

/// Arguments for `list_processes` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListProcessesArgs {
    /// Optional: filter by name (case-insensitive substring match)
    #[serde(default)]
    pub filter: Option<String>,

    /// Maximum number of processes to return (0 = unlimited)
    #[serde(default)]
    pub limit: usize,
}

/// Prompt arguments for `list_processes` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListProcessesPromptArgs {}

// ============================================================================
// KILL PROCESS
// ============================================================================

/// Arguments for `kill_process` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KillProcessArgs {
    /// Process ID to terminate
    pub pid: u32,
}

/// Prompt arguments for `kill_process` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KillProcessPromptArgs {}
