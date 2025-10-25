//! Core types and constants for the generation module
//!
//! This module defines fundamental types and constants used throughout
//! the generation system, providing a centralized location for shared
//! type definitions and configuration constants.

use smallvec::SmallVec;

use crate::domain::model::error::CandleModelError as CandleError;

/// Result type alias for generation operations
pub type CandleResult<T> = Result<T, CandleError>;

/// Buffer type for logits processing with stack optimization
///
/// Uses SmallVec to avoid heap allocation for typical vocab sizes.
/// SAMPLING_CACHE_SIZE chosen to accommodate most common tokenizer vocabularies
/// without triggering heap allocation.
pub type LogitsBuffer = SmallVec<f32, SAMPLING_CACHE_SIZE>;

/// Cache size for SIMD sampling operations
///
/// Optimized for typical transformer vocabulary sizes (30k-100k tokens).
/// This size balances memory usage with performance for stack allocation.
pub const SAMPLING_CACHE_SIZE: usize = 1024;

/// Minimum vector length threshold for SIMD operations
///
/// Below this threshold, scalar operations may be more efficient
/// due to SIMD setup overhead. Value determined through benchmarking.
pub const SIMD_THRESHOLD: usize = 64;

/// Default maximum context length for token history
///
/// Used for repetition penalty calculations and context tracking.
/// Balances memory usage with effective repetition detection.
pub const DEFAULT_CONTEXT_LENGTH: usize = 2048;

/// Default batch size for processing operations
///
/// Optimized for typical GPU/CPU memory hierarchies and
/// transformer model characteristics.
pub const DEFAULT_BATCH_SIZE: usize = 1;
