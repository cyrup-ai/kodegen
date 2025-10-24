//! Candle completion module - Consolidated completion functionality
//!
//! This module consolidates Candle completion-related functionality from completion.rs and `candle_completion.rs`
//! into a clean, unified module structure with zero-allocation patterns and production-ready functionality.
//!
//! ## Architecture
//! - `core.rs` - Core Candle completion traits and domain types
//! - `request.rs` - Candle completion request types and builders
//! - `response.rs` - Candle completion response types and builders
//! - `candle.rs` - Zero-allocation, lock-free Candle completion system
//! - `types.rs` - Shared Candle types and constants

pub mod candle;
// pub mod chunk; // Module not yet implemented
pub mod core;
pub mod prompt_formatter;
pub mod request;
pub mod response;
mod tool_formatter;
mod tool_parser;
pub mod types;

// Re-export commonly used Candle types for convenience
pub use candle::{
    CompletionCoreError, CompletionCoreRequest, CompletionCoreResponse, CompletionCoreResult,
    StreamingCoreResponse,
};
pub use prompt_formatter::PromptFormatter;
pub use tool_formatter::format_tools_for_qwen3;
pub use tool_parser::{ToolCall, ToolCallParser};

// Type aliases for convenience
pub type CandleCompletionResult<T> = CompletionCoreResult<T>;
pub type CandleStreamingResponse = StreamingCoreResponse;
pub use request::{CompletionRequest, CompletionRequestError};
pub type CandleCompletionRequest = CompletionRequest;
pub type CandleCompletionRequestError = CompletionRequestError;
pub use response::{CompactCompletionResponse, CompletionResponse};
pub type CandleCompactCompletionResponse = CompactCompletionResponse;
pub type CandleCompletionResponse<'a> = CompletionResponse<'a>;
pub use types::{CandleCompletionParams, CandleModelParams};

// Re-export CandleCompletionChunk from context/chunk.rs
pub use crate::domain::context::chunks::CandleCompletionChunk;
