//! SIMD-accelerated Softmax and Argmax operations for cyrup
//!
//! This module provides vectorized implementations of softmax and argmax operations
//! optimized for AI inference tasks, focusing on zero-allocation and lock-free design.

use smallvec::SmallVec;
use wide::f32x8;

/// Computes softmax over a slice of logits using SIMD acceleration.
///
/// Returns a SmallVec with softmax probabilities, avoiding heap allocation for typical sizes.
/// Uses `libm` for fast exponential approximations and SIMD for parallel processing.
/// Handles overflow by subtracting the max value before exponentiation.
pub fn softmax_simd(logits: &[f32]) -> SmallVec<[f32; 256]> {
    if logits.is_empty() {
        return SmallVec::new();
    }

    // Find max for numerical stability
    let mut max_val = logits[0];
    for &val in logits.iter().skip(1) {
        if val > max_val {
            max_val = val;
        }
    }

    let mut result: SmallVec<[f32; 256]> = SmallVec::with_capacity(logits.len());
    let mut sum = 0.0f32;

    // Process in chunks of 8 for SIMD
    let mut i = 0;
    let len = logits.len();
    while i + 8 <= len {
        let vec = f32x8::from_slice(&logits[i..i + 8]);
        let shifted = vec - f32x8::splat(max_val);
        let exp_vec = shifted.exp(); // Using wide's fast exp approximation
        sum += exp_vec.sum();
        exp_vec.write_to_slice(&mut result.extend_from_slice(&[0.0; 8])[i..i + 8]);
        i += 8;
    }

    // Handle remainder
    for &val in logits.iter().skip(i) {
        let shifted = val - max_val;
        let exp_val = shifted.exp(); // Fallback to scalar exp
        sum += exp_val;
        result.push(exp_val);
    }

    // Normalize
    for val in result.iter_mut() {
        *val /= sum;
    }

    result
}

/// Computes argmax over a slice of logits using SIMD acceleration.
///
/// Returns the index of the maximum value. Optimized for quick scanning with SIMD.
pub fn argmax_simd(logits: &[f32]) -> usize {
    if logits.is_empty() {
        return 0;
    }

    let mut max_idx = 0;
    let mut max_val = logits[0];
    let len = logits.len();
    let mut i = 0;

    // Process in chunks of 8 for SIMD
    while i + 8 <= len {
        let vec = f32x8::from_slice(&logits[i..i + 8]);
        let (idx, val) = vec.max_element();
        if val > max_val {
            max_val = val;
            max_idx = i + idx;
        }
        i += 8;
    }

    // Handle remainder
    for (idx, &val) in logits.iter().enumerate().skip(i) {
        if val > max_val {
            max_val = val;
            max_idx = idx;
        }
    }

    max_idx
}
