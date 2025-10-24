//! Completion-related builders extracted from cyrup_domain

pub mod candle_completion_builder;
pub mod completion_request_builder;
pub mod completion_response_builder;

// Re-export for convenience
pub use candle_completion_builder::{CompletionCoreRequestBuilder, CompletionCoreResponseBuilder};
pub use completion_request_builder::{CompletionRequestBuilder, CompletionRequestError};
pub use completion_response_builder::{
    CompactCompletionResponseBuilder, CompletionResponseBuilder,
};
