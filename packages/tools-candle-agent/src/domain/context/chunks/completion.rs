//! Completion chunk types for streaming AI completions
//!
//! This module contains types for streaming completions including:
//! - Text chunks
//! - Tool calls (start, partial, complete)
//! - Finish reasons
//! - Usage information

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};

use crate::domain::model::CandleUsage;

/// Reason why a completion finished
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FinishReason {
    /// Completion finished naturally at a stopping point
    Stop,
    /// Completion reached maximum token length limit
    Length,
    /// Completion was filtered due to content policy
    ContentFilter,
    /// Completion finished to execute tool calls
    ToolCalls,
    /// Completion failed due to an error
    Error,
}

/// Comprehensive completion chunk supporting all streaming features - EXACT REPLICA of domain
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CandleCompletionChunk {
    /// Text content chunk
    Text(String),

    /// Tool call started
    ToolCallStart { id: String, name: String },

    /// Partial tool call with streaming input
    ToolCall {
        id: String,
        name: String,
        partial_input: String,
    },

    /// Tool call completed
    ToolCallComplete {
        id: String,
        name: String,
        input: String,
    },

    /// Completion finished with final information
    Complete {
        text: String,
        finish_reason: Option<FinishReason>,
        usage: Option<CandleUsage>,
        token_count: Option<u32>,
        elapsed_secs: Option<f64>,
        tokens_per_sec: Option<f64>,
    },

    /// Error occurred during streaming
    Error(String),
}

impl Default for CandleCompletionChunk {
    fn default() -> Self {
        CandleCompletionChunk::Text(String::new())
    }
}

impl MessageChunk for CandleCompletionChunk {
    fn bad_chunk(error: String) -> Self {
        CandleCompletionChunk::Error(error)
    }

    fn error(&self) -> Option<&str> {
        match self {
            CandleCompletionChunk::Error(err) => Some(err),
            _ => None,
        }
    }
}

/// Chunk of chat message for streaming responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageChunk {
    /// Partial message content
    pub content: String,

    /// Role of the message sender
    pub role: crate::domain::chat::message::types::CandleMessageRole,

    /// Whether this is the final chunk
    pub is_final: bool,

    /// Additional metadata
    #[serde(flatten)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl MessageChunk for ChatMessageChunk {
    fn bad_chunk(error: String) -> Self {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("error".to_string(), serde_json::Value::String(error));
        Self {
            content: String::new(),
            role: crate::domain::chat::message::types::CandleMessageRole::User,
            is_final: false,
            metadata,
        }
    }

    fn error(&self) -> Option<&str> {
        if let Some(serde_json::Value::String(error)) = self.metadata.get("error") {
            Some(error)
        } else {
            None
        }
    }
}
