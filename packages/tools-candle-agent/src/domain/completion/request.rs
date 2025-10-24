//! Completion request types and builders
//!
//! Contains request structures and builder patterns for completion functionality.

// Removed unused import: std::borrow::Cow
use std::num::NonZeroU64;

use cyrup_sugars::ZeroOneOrMany;
use serde_json::Value;
use thiserror::Error;

use super::types::{MAX_CHUNK_SIZE, MAX_TOKENS, TEMPERATURE_RANGE};
use crate::domain::chat::message::types::CandleMessage as ChatMessage;
use crate::domain::context::CandleDocument as Document;
use crate::memory::core::ops::retrieval::RetrievalResult;
use rmcp::model::Tool as ToolInfo;
// NOTE: ToolDefinition was removed with HTTP infrastructure cleanup
// Local Candle tool definitions will be implemented when needed
use crate::domain::model::{ValidationError, ValidationResult};

/// A request for text completion
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// System prompt providing instructions
    pub system_prompt: String,
    /// Conversation history
    pub chat_history: ZeroOneOrMany<ChatMessage>,
    /// Documents to use as context
    pub documents: ZeroOneOrMany<Document>,
    /// Retrieved memories from previous conversations
    pub memories: ZeroOneOrMany<RetrievalResult>,
    /// Tools available to the model
    pub tools: ZeroOneOrMany<ToolInfo>,
    /// Sampling temperature (0.0 to 2.0)
    pub temperature: f64,
    /// Maximum number of tokens to generate
    pub max_tokens: Option<NonZeroU64>,
    /// Size of chunks for streaming
    pub chunk_size: Option<usize>,
    /// Additional provider-specific parameters
    pub additional_params: Option<Value>,
}

/// Error type for completion request validation
#[derive(Debug, Error)]
pub enum CompletionRequestError {
    /// Invalid parameter value
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Validation error
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

impl CompletionRequest {
    /// Validate the request parameters
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if:
    /// - Temperature is outside valid range
    /// - Other request parameters fail validation
    pub fn validate(&self) -> ValidationResult<()> {
        // Validate temperature
        if !TEMPERATURE_RANGE.contains(&self.temperature) {
            return Err(ValidationError::InvalidRange {
                field: "temperature".into(),
                value: self.temperature.to_string(),
                expected: format!(
                    "between {:.1} and {:.1}",
                    TEMPERATURE_RANGE.start(),
                    TEMPERATURE_RANGE.end()
                ),
            });
        }

        // Validate max_tokens
        if let Some(max_tokens) = self.max_tokens
            && max_tokens.get() > MAX_TOKENS
        {
            return Err(ValidationError::InvalidRange {
                field: "max_tokens".into(),
                value: max_tokens.to_string(),
                expected: format!("less than or equal to {MAX_TOKENS}"),
            });
        }

        // Validate chunk_size
        if let Some(chunk_size) = self.chunk_size
            && (chunk_size == 0 || chunk_size > MAX_CHUNK_SIZE)
        {
            return Err(ValidationError::InvalidRange {
                field: "chunk_size".into(),
                value: chunk_size.to_string(),
                expected: format!("between 1 and {MAX_CHUNK_SIZE}"),
            });
        }

        Ok(())
    }

    /// Convert to a static lifetime version by making all borrowed data owned
    #[must_use]
    #[inline]
    pub fn into_static(self) -> CompletionRequest {
        CompletionRequest {
            system_prompt: self.system_prompt,
            chat_history: self.chat_history,
            documents: self.documents,
            memories: self.memories,
            tools: self.tools,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            chunk_size: self.chunk_size,
            additional_params: self.additional_params,
        }
    }
}
