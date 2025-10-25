// Tests extracted from src/core/generation/types.rs

use kodegen_candle_agent::core::generation::types::*;
use smallvec::SmallVec;

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
