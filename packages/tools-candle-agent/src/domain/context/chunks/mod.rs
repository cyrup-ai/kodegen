//! Chunk Types for Streaming Operations
//!
//! This module contains all chunk types that represent partial data flowing through
//! `AsyncStream<T>` and are designed to work with the `NotResult` constraint.
//!
//! ## Module Organization
//!
//! - **media**: Media-related chunks (documents, images, audio, transcription, speech)
//! - **completion**: Completion chunks for AI streaming responses
//! - **`generic_wrappers`**: Generic wrapper types for common operations
//! - **`result_types`**: Result types for operations (`CandleResult`, `ParallelResult`, etc.)
//! - **`primitive_wrappers`**: Wrappers for primitive types to satisfy orphan rules

// Module declarations
pub mod completion;
pub mod generic_wrappers;
pub mod media;
pub mod primitive_wrappers;
pub mod result_types;

// Re-export all public types for backward compatibility
pub use completion::{CandleCompletionChunk, ChatMessageChunk, FinishReason};
pub use generic_wrappers::{
    CandleCollectionChunk, CandleJsonChunk, CandleStringChunk, CandleUnitChunk, EmbeddingChunk,
    GenerationStats, WorkflowDataChunk,
};
pub use media::{
    AudioFormat, CandleDocumentChunk, CandleImageChunk, CandleImageFormat, SpeechChunk,
    TranscriptionChunk, VoiceChunk,
};
pub use primitive_wrappers::{
    CandleBoolChunk, CandleDateTimeChunk, CandleDurationChunk, CandleUnit, CandleUuidChunk,
    CandleZeroOneOrManyChunk, ZeroOneOrManyF32Chunk,
};
pub use result_types::{
    CandleMemoryOperationResult, CandleRefreshResult, CandleResult, ParallelResult,
};

// Note: Orphan rule violations removed - use wrapper types instead:
// - Use CandleUnitChunk for ()
// - Use CandleStringChunk for String
// - Use CandleUuidChunk for Uuid
// - Use CandleBoolChunk for bool
// - Use CandleDurationChunk for Duration
// - Use CandleDateTimeChunk for DateTime<Utc>
// - Use ZeroOneOrManyF32Chunk for ZeroOneOrMany<f32>
