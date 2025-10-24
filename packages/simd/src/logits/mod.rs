//! Logits processing module for SIMD-accelerated operations

use smallvec::SmallVec;

pub mod constraints;
mod nucleus;
mod penalties;
pub mod processing;
pub mod processor;
pub mod simd;
pub mod topk;

pub use nucleus::*;
pub use penalties::*;
pub use processing::*;
// processor module exports DefaultLogitsProcessor
// simd module provides compatibility aliases
pub use topk::topk_filtering_simd;

use crate::config::ProcessorConfig;
use crate::context::ProcessingContext;

/// Error type for logits processing operations
#[derive(Debug, thiserror::Error)]
pub enum LogitsError {
    /// Invalid input length for logits processing
    #[error("Invalid input length: {0}")]
    InvalidInputLength(usize),

    /// Numerical computation error during processing
    #[error("Numerical error: {0}")]
    NumericalError(String),

    /// Unsupported operation requested
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Error during sampling process
    #[error("Sampling error: {0}")]
    SamplingError(String),

    /// Configuration validation error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Constraint validation error during processing
    #[error("Constraint error: {0}")]
    ConstraintError(String),

    /// SIMD processing error
    #[error("SIMD error: {0}")]
    SimdError(#[from] crate::error::SimdError),
}

/// Result type for logits processing operations
pub type LogitsResult<T> = Result<T, LogitsError>;

/// Trait for logits processing operations
pub trait LogitsProcessor: Send + Sync {
    /// Process logits in-place
    fn process(&mut self, logits: &mut [f32], context: &ProcessingContext) -> LogitsResult<()>;

    /// Get the current configuration
    fn config(&self) -> &ProcessorConfig;

    /// Get a mutable reference to the configuration
    fn config_mut(&mut self) -> &mut ProcessorConfig;
}

// DefaultLogitsProcessor is implemented in the processor module

/// Process logits using scalar operations (fallback when SIMD is not available)
pub fn process_logits_scalar(
    logits: &mut [f32],
    context: &ProcessingContext,
    _config: &ProcessorConfig,
) -> LogitsResult<()> {
    // Apply temperature scaling
    let temp = context.temperature;
    if temp != 1.0 && temp > 0.0 {
        let inv_temp = 1.0 / temp;
        for x in logits.iter_mut() {
            *x *= inv_temp;
        }
    }

    // Apply top-k filtering if enabled
    if let Some(k) = context.top_k
        && k < logits.len()
    {
        // Use partial sort to find the k-th largest element efficiently
        let mut sorted: Vec<f32> = logits.to_vec();

        if k > 0 && k < sorted.len() {
            // Find the k-th largest element position
            let kth = sorted.len() - k;
            sorted.select_nth_unstable_by(kth, |a, b| {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Get threshold value at the k-th position
            let threshold = sorted[kth];

            // Mask all values below threshold
            for x in logits.iter_mut() {
                if *x < threshold {
                    *x = f32::NEG_INFINITY;
                }
            }
        }
    }

    // Apply nucleus sampling if enabled
    if let Some(p) = context.top_p
        && p > 0.0
        && p < 1.0
    {
        // Find max logit for numerical stability (zero allocation)
        let max_logit = logits.iter().fold(f32::NEG_INFINITY, |acc, &x| acc.max(x));

        // Create SmallVec of (index, logit) pairs
        let mut sorted: SmallVec<(usize, f32), 512> =
            logits.iter().enumerate().map(|(i, &v)| (i, v)).collect();

        // Sort in descending order by logit value
        sorted.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Compute total sum of exp(shifted) without allocation
        let mut total_sum = 0.0f64;
        for &logit in logits.iter() {
            total_sum += ((logit - max_logit) as f64).exp();
        }

        // Now find cutoff using cumulative normalized prob
        let mut cumsum = 0.0f64;
        let mut cutoff = sorted.len();

        for (i, &(_, logit)) in sorted.iter().enumerate() {
            let prob = ((logit - max_logit) as f64).exp() / total_sum;
            cumsum += prob;
            if cumsum >= p as f64 {
                cutoff = i + 1;
                break;
            }
        }

        // Collect indices to keep (using SmallVec to avoid alloc if small)
        let keep_indices: SmallVec<usize, 512> =
            sorted[..cutoff].iter().map(|&(idx, _)| idx).collect();

        // Mask logits not in keep (in-place, zero alloc)
        for (i, logit) in logits.iter_mut().enumerate() {
            if !keep_indices.contains(&i) {
                *logit = f32::NEG_INFINITY;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_temperature_scaling() {
        let mut logits = vec![1.0, 2.0, 3.0];
        let context = ProcessingContext::new().with_temperature(0.5);
        let config = ProcessorConfig::default();

        if let Err(e) = process_logits_scalar(&mut logits, &context, &config) {
            panic!("Temperature scaling failed: {}", e);
        }
        assert_relative_eq!(logits[0], 2.0);
        assert_relative_eq!(logits[1], 4.0);
        assert_relative_eq!(logits[2], 6.0);
    }

    #[test]
    fn test_topk_filtering() {
        let mut logits = vec![3.0, 1.0, 4.0, 1.0, 5.0];
        let context = ProcessingContext::new().with_top_k(Some(2));
        let config = ProcessorConfig::default();

        if let Err(e) = process_logits_scalar(&mut logits, &context, &config) {
            panic!("Top-k filtering failed: {}", e);
        }

        // Only top 2 values should remain non-negative infinity
        let non_inf = logits.iter().filter(|&&x| x > f32::NEG_INFINITY).count();
        assert_eq!(non_inf, 2);
        // Check specific values
        let mut sorted = logits.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        assert!(logits.contains(&sorted[0]));
        assert!(logits.contains(&sorted[1]));
    }

    #[test]
    fn test_nucleus_sampling() {
        let mut logits = vec![0.1, 0.2, 0.3, 0.4];
        let context = ProcessingContext::new().with_top_p(Some(0.5));
        let config = ProcessorConfig::default();

        if let Err(e) = process_logits_scalar(&mut logits, &context, &config) {
            panic!("Nucleus sampling failed: {}", e);
        }

        // Verify correct masking (indices 0 and 1 masked for top_p=0.5)
        let has_inf = logits.iter().filter(|&&x| x == f32::NEG_INFINITY).count() == 2;
        assert!(has_inf);
        assert_eq!(logits[0], f32::NEG_INFINITY);
        assert_eq!(logits[1], f32::NEG_INFINITY);
        assert!(logits[2] > f32::NEG_INFINITY);
        assert!(logits[3] > f32::NEG_INFINITY);
    }
}
