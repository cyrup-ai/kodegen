//! Agent definition and system prompt types
//!
//! This module contains types for defining agents and configuring system prompts.

use serde::{Deserialize, Serialize};

// ============================================================================
// System Prompt Types
// ============================================================================

/// System prompt preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPromptPreset {
    /// Prompt type (always "preset")
    #[serde(rename = "type")]
    pub prompt_type: String,
    /// Preset name (e.g., "`claude_code`")
    pub preset: String,
    /// Additional text to append to the preset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
}

/// System prompt configuration
#[derive(Debug, Clone)]
pub enum SystemPrompt {
    /// Plain string system prompt
    String(String),
    /// Preset-based system prompt
    Preset(SystemPromptPreset),
}

// Implement conversions for SystemPrompt
impl From<String> for SystemPrompt {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for SystemPrompt {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<SystemPromptPreset> for SystemPrompt {
    fn from(preset: SystemPromptPreset) -> Self {
        Self::Preset(preset)
    }
}

// ============================================================================
// Agent Definition
// ============================================================================

/// Agent definition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent description
    pub description: String,
    /// Agent system prompt
    pub prompt: String,
    /// Tools available to the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Model to use for the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

// ============================================================================
// SESSION MANAGEMENT TYPES (Added for agent session tools)
// ============================================================================

use chrono::{DateTime, Utc};

/// Serialized message stored in agent session circular buffer
///
/// Flattens Message enum variants into storable JSON format for efficient
/// retrieval and pagination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMessage {
    /// Message type: "assistant", "user", "system_<subtype>", "result", "`stream_event`"
    pub message_type: String,

    /// Full message content as JSON Value
    /// Preserves all fields from original Message enum variant
    pub content: serde_json::Value,

    /// Turn number (extracted from Result message, 0 for others)
    pub turn: u32,

    /// When this message was received by session manager
    pub timestamp: DateTime<Utc>,
}

/// Response from `get_output` (paginated agent message output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOutputResponse {
    /// Unique identifier for the agent session
    pub session_id: String,

    /// TRUE if agent actively processing (recent message activity)
    /// FALSE if idle, complete, or at `max_turns`
    pub working: bool,

    /// Messages in this page (`SerializedMessage` array)
    pub output: Vec<SerializedMessage>,

    /// Total messages currently in circular buffer
    pub total_messages: usize,

    /// Number of messages returned in this response
    pub messages_returned: usize,

    /// Conversation complete (Result message received OR `max_turns` reached)
    pub is_complete: bool,

    /// Current turn count (from last Result message)
    pub turn_count: u32,

    /// Maximum turns configured for this session
    pub max_turns: u32,

    /// More messages available for pagination
    /// TRUE if: offset+length < `total_messages` (for positive offset)
    /// FALSE if: reading tail OR no more messages
    pub has_more: bool,
}

/// Response from `terminate_session`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminateResponse {
    /// Unique identifier for the terminated session
    pub session_id: String,
    /// Whether the session was successfully terminated
    pub success: bool,

    /// Final turn count when session was terminated
    pub final_turn_count: u32,

    /// Total messages collected during session lifetime
    pub total_messages: usize,

    /// Session runtime in milliseconds
    pub runtime_ms: u64,
}

/// Agent session info for `list_sessions` response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique identifier for the agent session
    pub session_id: String,

    /// User-provided label for session identification
    pub label: String,

    /// TRUE if actively processing (recent message activity)
    pub working: bool,

    /// Current turn count
    pub turn_count: u32,

    /// Maximum turns configured
    pub max_turns: u32,

    /// Session runtime in milliseconds
    pub runtime_ms: u64,

    /// Total messages in buffer
    pub message_count: usize,

    /// TRUE if session completed (Result received OR `max_turns` reached)
    pub is_complete: bool,

    /// Last N lines of assistant output for preview
    /// Extracted from assistant message content blocks
    pub last_output: Vec<String>,

    /// When session completed (None if still active)
    pub completion_time: Option<DateTime<Utc>>,
}

/// Response from `list_sessions`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    /// All sessions (active + completed if requested)
    /// Sorted by: working=true first, then by runtime (newest first)
    pub agents: Vec<AgentInfo>,

    /// Count of active sessions (`is_complete=false`)
    pub total_active: usize,

    /// Count of completed sessions (`is_complete=true`)
    pub total_completed: usize,
}
