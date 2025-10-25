// Tests extracted from src/core/simd_adapters.rs

use kodegen_candle_agent::core::simd_adapters::{
    simd_temperature_scale, should_use_simd, simd_argmax_with_bounds,
};
use kodegen_candle_agent::core::generation::types::LogitsBuffer;
use smallvec::smallvec;
use arrayvec::ArrayVec;

#[test]
fn test_temperature_scale_empty_logits() {
    let mut logits = LogitsBuffer::new();
    let result = simd_temperature_scale(&mut logits, 1.5);
    assert!(result.is_err());
}

#[test]
fn test_temperature_scale_invalid_temperature() {
    let mut logits = smallvec![1.0, 2.0, 3.0];
    let result = simd_temperature_scale(&mut logits, 0.0);
    assert!(result.is_err());
}

#[test]
fn test_should_use_simd_conditions() {
    assert!(should_use_simd(100, 50, true));
    assert!(!should_use_simd(30, 50, true));
    assert!(!should_use_simd(100, 50, false));
}

#[test]
fn test_argmax_empty_probabilities() {
    let probabilities: &[f32] = &[];
    let prob_cache = ArrayVec::new();
    let result = simd_argmax_with_bounds(probabilities, &prob_cache);
    assert!(result.is_err());
}

#[test]
fn test_argmax_empty_cache() {
    let probabilities = &[0.1, 0.8, 0.1];
    let prob_cache = ArrayVec::new();
    let result = simd_argmax_with_bounds(probabilities, &prob_cache);
    assert!(result.is_err());
}
