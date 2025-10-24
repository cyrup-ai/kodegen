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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_constants_are_reasonable() {
        assert!(SAMPLING_CACHE_SIZE > 0);
        assert!(SIMD_THRESHOLD > 0);
        assert!(DEFAULT_CONTEXT_LENGTH > 0);
        assert!(DEFAULT_BATCH_SIZE > 0);

        // Ensure SIMD threshold is smaller than cache size
        assert!(SIMD_THRESHOLD < SAMPLING_CACHE_SIZE);
    }

    #[test]
    fn test_logits_buffer_creation() {
        let buffer: LogitsBuffer = SmallVec::new();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.capacity() >= SAMPLING_CACHE_SIZE);
    }
}
