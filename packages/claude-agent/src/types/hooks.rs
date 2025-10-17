//! Hook-related type definitions
//!
//! This module contains types for managing hooks, including hook events,
//! hook decisions, hook outputs, and hook callbacks.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Result;

// ============================================================================
// Hook Types
// ============================================================================

/// Hook event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Before a tool is used
    PreToolUse,
    /// After a tool is used
    PostToolUse,
    /// When user submits a prompt
    UserPromptSubmit,
    /// When conversation stops
    Stop,
    /// When a subagent stops
    SubagentStop,
    /// Before compacting the conversation
    PreCompact,
}

/// Hook decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookDecision {
    /// Block the action
    Block,
}

/// Hook output
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookOutput {
    /// Decision to block or allow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<HookDecision>,
    /// System message to add
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemMessage")]
    pub system_message: Option<String>,
    /// Hook-specific output data
    #[serde(skip_serializing_if = "Option::is_none", rename = "hookSpecificOutput")]
    pub hook_specific_output: Option<serde_json::Value>,
}

/// Context for hook callbacks
#[derive(Debug, Clone)]
pub struct HookContext {
    // Future: abort signal support
}

/// Hook callback type
pub type HookCallback = Arc<
    dyn Fn(
            serde_json::Value,
            Option<String>,
            HookContext,
        ) -> Pin<Box<dyn Future<Output = Result<HookOutput>> + Send>>
        + Send
        + Sync,
>;

/// Hook matcher configuration
#[derive(Clone)]
pub struct HookMatcher {
    /// Matcher pattern (e.g., tool name like "Bash" or pattern like "Write|Edit")
    pub matcher: Option<String>,
    /// List of hook callbacks
    pub hooks: Vec<HookCallback>,
}

impl std::fmt::Debug for HookMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookMatcher")
            .field("matcher", &self.matcher)
            .field("hooks", &format!("[{} callbacks]", self.hooks.len()))
            .finish()
    }
}
