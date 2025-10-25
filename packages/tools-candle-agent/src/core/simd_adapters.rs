//! SIMD Adapter Module for Candle
//!
//! Bridge functions between kodegen_simd API and generation.rs data structures.
//! Uses the existing kodegen_simd operations directly without duplication.

use arrayvec::ArrayVec;
use kodegen_simd::{SimdError, argmax, scale_temperature, softmax};

use crate::core::generation::{CandleResult, LogitsBuffer, TokenProb};
use crate::domain::model::error::CandleModelError as CandleError;

// Use the constant from generation.rs - define locally to avoid privacy issues
const SAMPLING_CACHE_SIZE: usize = 1024;

/// SIMD-optimized temperature scaling for LogitsBuffer (SmallVec)
///
/// # Arguments
/// * `logits` - Mutable reference to logits buffer
/// * `temperature` - Temperature scaling factor
///
/// # Returns
/// * `CandleResult<()>` - Success or error with fallback recommendation
pub fn simd_temperature_scale(logits: &mut LogitsBuffer, temperature: f32) -> CandleResult<()> {
    // Validate inputs before delegating to SIMD layer
    if logits.is_empty() {
        return Err(CandleError::InvalidInput(
            "Cannot scale temperature on empty logits buffer".into(),
        ));
    }

    match scale_temperature(logits.as_mut_slice(), temperature) {
        Ok(()) => Ok(()),
        Err(simd_err) => {
            // Convert SIMD error to Candle error with context
            Err(CandleError::Internal(
                format!("SIMD temperature scaling failed: {}", simd_err).into(),
            ))
        }
    }
}

/// SIMD-optimized softmax with TokenProb cache integration
///
/// # Arguments
/// * `logits` - Input logits buffer
/// * `prob_cache` - Output cache for TokenProb results
///
/// # Returns
/// * `CandleResult<Vec<f32>>` - Computed probabilities or error
pub fn simd_softmax_with_cache(
    logits: &LogitsBuffer,
    prob_cache: &mut ArrayVec<TokenProb, SAMPLING_CACHE_SIZE>,
) -> CandleResult<Vec<f32>> {
    // Compute SIMD softmax using existing kodegen_simd function
    let probabilities = match softmax(logits.as_slice()) {
        Ok(probs) => probs,
        Err(simd_err) => {
            return Err(CandleError::Internal(
                format!("SIMD softmax failed: {}", simd_err).into(),
            ));
        }
    };

    // Populate prob_cache with results
    prob_cache.clear();
    for (token_id, &prob) in probabilities.iter().enumerate() {
        if prob_cache
            .try_push(TokenProb::new(token_id as u32, prob))
            .is_err()
        {
            // Cache full, use what we have
            break;
        }
    }

    Ok(probabilities)
}

/// SIMD-optimized argmax with bounds checking
///
/// # Arguments
/// * `probabilities` - Input probability slice
/// * `prob_cache` - Associated TokenProb cache for token mapping
///
/// # Returns
/// * `CandleResult<u32>` - Token ID of maximum probability or error
pub fn simd_argmax_with_bounds(
    probabilities: &[f32],
    prob_cache: &ArrayVec<TokenProb, SAMPLING_CACHE_SIZE>,
) -> CandleResult<u32> {
    // Compute SIMD argmax using existing kodegen_simd function
    let max_idx = match argmax(probabilities) {
        Ok(idx) => idx,
        Err(simd_err) => {
            return Err(CandleError::Internal(
                format!("SIMD argmax failed: {}", simd_err).into(),
            ));
        }
    };

    // Bounds checking
    if max_idx >= prob_cache.len() {
        return Err(CandleError::InvalidInput(
            format!(
                "Argmax index {} out of bounds for cache size {}",
                max_idx,
                prob_cache.len()
            )
            .into(),
        ));
    }

    if max_idx >= probabilities.len() {
        return Err(CandleError::InvalidInput(
            format!(
                "Argmax index {} out of bounds for probability array size {}",
                max_idx,
                probabilities.len()
            )
            .into(),
        ));
    }

    // Return token_id from cache
    Ok(prob_cache[max_idx].token_id)
}

/// Utility function to check if SIMD should be used based on array size and configuration
///
/// # Arguments
/// * `array_size` - Size of the array to process
/// * `simd_threshold` - Minimum size threshold for SIMD usage
/// * `simd_enabled` - Whether SIMD is enabled in configuration
///
/// # Returns
/// * `bool` - True if SIMD should be used
pub fn should_use_simd(array_size: usize, simd_threshold: usize, simd_enabled: bool) -> bool {
    simd_enabled && array_size >= simd_threshold && kodegen_simd::simd_available()
}

/// Convert SIMD error to appropriate fallback strategy description
///
/// # Arguments
/// * `simd_error` - The SIMD error that occurred
///
/// # Returns
/// * `String` - Human-readable fallback recommendation
pub fn simd_error_to_fallback_strategy(simd_error: &SimdError) -> String {
    match simd_error {
        SimdError::UnsupportedOperation(msg) => {
            format!("Use scalar fallback - SIMD not supported: {}", msg)
        }
        SimdError::InvalidInput(msg) => format!("Use scalar fallback - Invalid input: {}", msg),
        SimdError::InvalidInputLength { expected, actual } => format!(
            "Use scalar fallback - Invalid length: expected {}, got {}",
            expected, actual
        ),
        SimdError::MemoryError(msg) => format!("Use scalar fallback - Memory error: {}", msg),
        SimdError::ProcessingError(msg) => {
            format!("Use scalar fallback - Processing error: {}", msg)
        }
        SimdError::NumericalError(msg) => format!("Use scalar fallback - Numerical error: {}", msg),
        _ => "Use scalar fallback - SIMD operation failed".to_string(),
    }
}
