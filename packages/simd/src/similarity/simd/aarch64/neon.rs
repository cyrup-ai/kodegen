//! Real ARM NEON SIMD implementation for AArch64 architectures
//!
//! This module provides production-quality NEON intrinsics for efficient
//! cosine similarity calculations with zero allocation patterns.

use std::sync::Arc;

use crate::similarity::metrics::{MetricsGuard, SimilarityMetrics, SimilarityMetricsSnapshot};
use crate::similarity::traits::{CosineSimilarity, RuntimeSelectable, WithMetrics};

/// NEON-optimized similarity implementation for ARM64
pub struct NeonSimilarity {
    metrics: Arc<SimilarityMetrics>,
}

impl Default for NeonSimilarity {
    fn default() -> Self {
        Self::new()
    }
}

impl NeonSimilarity {
    /// Create a new NEON similarity instance
    #[inline]
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(SimilarityMetrics::default()),
        }
    }

    /// Process vectors using real ARM NEON SIMD instructions
    #[target_feature(enable = "neon")]
    unsafe fn process_neon_chunks(a: &[f32], b: &[f32]) -> (f32, f32, f32) {
        use std::arch::aarch64::*;

        let len = a.len();
        let chunks = len / 4; // Process 4 f32 values at a time with NEON
        let remainder = len % 4;

        // Initialize SIMD accumulators
        let mut dot_sum = vdupq_n_f32(0.0);
        let mut a_norm_sum = vdupq_n_f32(0.0);
        let mut b_norm_sum = vdupq_n_f32(0.0);

        // Process 4 floats at a time using real NEON intrinsics
        for i in 0..chunks {
            let offset = i * 4;

            // Load 4 consecutive f32 values from each vector
            let a_vec = unsafe { vld1q_f32(a.as_ptr().add(offset)) };
            let b_vec = unsafe { vld1q_f32(b.as_ptr().add(offset)) };

            // Fused multiply-add: dot_sum += a_vec * b_vec
            dot_sum = vfmaq_f32(dot_sum, a_vec, b_vec);

            // Compute squared norms: norm_a += a_vec * a_vec
            a_norm_sum = vfmaq_f32(a_norm_sum, a_vec, a_vec);

            // Compute squared norms: norm_b += b_vec * b_vec
            b_norm_sum = vfmaq_f32(b_norm_sum, b_vec, b_vec);
        }

        // Horizontal reduction of SIMD vectors to scalars
        let dot_scalar = unsafe { Self::horizontal_sum_neon(dot_sum) };
        let a_norm_scalar = unsafe { Self::horizontal_sum_neon(a_norm_sum) };
        let b_norm_scalar = unsafe { Self::horizontal_sum_neon(b_norm_sum) };

        // Process remainder elements with scalar operations
        let (dot_remainder, a_norm_remainder, b_norm_remainder) = if remainder > 0 {
            let remainder_offset = chunks * 4;
            Self::process_remainder_scalar(&a[remainder_offset..], &b[remainder_offset..])
        } else {
            (0.0, 0.0, 0.0)
        };

        (
            dot_scalar + dot_remainder,
            a_norm_scalar + a_norm_remainder,
            b_norm_scalar + b_norm_remainder,
        )
    }

    /// Horizontal sum of NEON vector using efficient reduction
    #[target_feature(enable = "neon")]
    #[inline]
    unsafe fn horizontal_sum_neon(vec: std::arch::aarch64::float32x4_t) -> f32 {
        use std::arch::aarch64::*;

        // Use NEON's horizontal add reduction
        vaddvq_f32(vec)
    }

    /// Process remainder elements using scalar operations
    #[inline]
    fn process_remainder_scalar(a: &[f32], b: &[f32]) -> (f32, f32, f32) {
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for i in 0..a.len() {
            let a_val = a[i];
            let b_val = b[i];

            dot += a_val * b_val;
            norm_a += a_val * a_val;
            norm_b += b_val * b_val;
        }

        (dot, norm_a, norm_b)
    }

    /// Check if the current vector length is suitable for NEON
    #[inline]
    fn is_suitable_length(len: usize) -> bool {
        len >= 4 // Need at least 4 elements for meaningful NEON usage
    }
}

impl CosineSimilarity for NeonSimilarity {
    #[inline]
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0; // Invalid input, return zero similarity
        }

        let _guard = MetricsGuard::new(&self.metrics, a.len());

        let (dot, norm_a, norm_b) = if Self::is_suitable_length(a.len()) && is_neon_available() {
            // Use real NEON SIMD intrinsics
            unsafe { Self::process_neon_chunks(a, b) }
        } else {
            // Fall back to scalar for very small vectors or when NEON unavailable
            Self::process_remainder_scalar(a, b)
        };

        // Compute cosine similarity with numerical stability
        let norm_product = (norm_a * norm_b).sqrt();
        if norm_product <= f32::EPSILON {
            0.0 // Handle zero vectors
        } else {
            // Clamp result to [-1, 1] to handle floating-point precision issues
            (dot / norm_product).clamp(-1.0, 1.0)
        }
    }
}

impl WithMetrics for NeonSimilarity {
    fn metrics(&self) -> SimilarityMetricsSnapshot {
        self.metrics.get_metrics()
    }

    fn reset_metrics(&self) {
        self.metrics.reset();
    }
}

impl RuntimeSelectable for NeonSimilarity {
    fn name(&self) -> &'static str {
        "neon"
    }

    fn optimal_vector_length(&self) -> usize {
        4 // Optimal for NEON f32x4 vectors
    }
}

/// Check if NEON is available at runtime
#[inline]
pub fn is_neon_available() -> bool {
    #[cfg(target_arch = "aarch64")]
    {
        // On AArch64, NEON is always available
        true
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_neon_cosine_similarity() {
        let sim = NeonSimilarity::new();

        // Test with 4-element vectors (perfect NEON alignment)
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.666_666_7; // Correct value: dot=20, norm=30, cosine=20/30=0.6667
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with 8-element vectors (two NEON chunks)
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.588_235_3; // Correct value: dot=120, norm=204, cosine=120/204=0.5882353
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with remainder elements (6 elements = 1 chunk + 2 remainder)
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.615_384_64; // Correct value: dot=56, norm=91, cosine=56/91=0.61538464
        assert_relative_eq!(result, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_edge_cases() {
        let sim = NeonSimilarity::new();

        // Test orthogonal vectors
        let a = [1.0, 0.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0, 0.0];
        let result = sim.cosine_similarity(&a, &b);
        assert_relative_eq!(result, 0.0, epsilon = 1e-6);

        // Test identical vectors
        let a = [1.0, 2.0, 3.0, 4.0];
        let result = sim.cosine_similarity(&a, &a);
        assert_relative_eq!(result, 1.0, epsilon = 1e-6);

        // Test zero vectors
        let a = [0.0; 4];
        let b = [1.0; 4];
        let result = sim.cosine_similarity(&a, &b);
        assert_eq!(result, 0.0);

        // Test small vectors (should use scalar fallback)
        let a = [1.0, 2.0];
        let b = [3.0, 4.0];
        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.983_869_9; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_metrics() {
        let sim = NeonSimilarity::new();
        sim.reset_metrics();

        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [4.0, 3.0, 2.0, 1.0];

        // First call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 1);
        assert_eq!(metrics.total_elements_processed, 4);

        // Second call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 2);
        assert_eq!(metrics.total_elements_processed, 8);
    }

    #[test]
    fn test_neon_availability() {
        // Just verify the function runs without panicking
        let _available = is_neon_available();

        // On AArch64, NEON should be available
        #[cfg(target_arch = "aarch64")]
        assert!(is_neon_available());

        // On other architectures, NEON should not be available
        #[cfg(not(target_arch = "aarch64"))]
        assert!(!is_neon_available());
    }
}
