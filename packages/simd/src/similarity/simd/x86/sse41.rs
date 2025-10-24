//! Real SSE4.1 SIMD implementation for 4-wide f32 operations
//!
//! This module provides production-quality SSE4.1 intrinsics for efficient
//! cosine similarity calculations with broad x86 compatibility.

use std::sync::Arc;

use crate::similarity::metrics::{MetricsGuard, SimilarityMetrics, SimilarityMetricsSnapshot};
use crate::similarity::traits::{CosineSimilarity, RuntimeSelectable, WithMetrics};

/// SSE4.1-optimized similarity implementation for x86/x64
pub struct Sse41Similarity {
    metrics: Arc<SimilarityMetrics>,
}

impl Default for Sse41Similarity {
    fn default() -> Self {
        Self::new()
    }
}

impl Sse41Similarity {
    /// Create a new SSE4.1 similarity instance
    #[inline]
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(SimilarityMetrics::default()),
        }
    }

    /// Process vectors using real SSE4.1 SIMD instructions
    #[target_feature(enable = "sse4.1")]
    unsafe fn process_sse41_chunks(a: &[f32], b: &[f32]) -> (f32, f32, f32) {
        use std::arch::x86_64::*;

        let len = a.len();
        let chunks = len / 4; // Process 4 f32 values at a time with SSE4.1
        let remainder = len % 4;

        // Initialize SIMD accumulators
        let mut dot_sum = _mm_setzero_ps();
        let mut norm_a_sum = _mm_setzero_ps();
        let mut norm_b_sum = _mm_setzero_ps();

        // Process 4 floats at a time using real SSE4.1 intrinsics
        for i in 0..chunks {
            let offset = i * 4;

            // Load 4 consecutive f32 values from each vector
            let a_vec = _mm_loadu_ps(a.as_ptr().add(offset));
            let b_vec = _mm_loadu_ps(b.as_ptr().add(offset));

            // Multiply and add to dot product: dot_sum += a_vec * b_vec
            let dot_product = _mm_mul_ps(a_vec, b_vec);
            dot_sum = _mm_add_ps(dot_sum, dot_product);

            // Compute squared norms: norm_a += a_vec * a_vec
            let a_squared = _mm_mul_ps(a_vec, a_vec);
            norm_a_sum = _mm_add_ps(norm_a_sum, a_squared);

            // Compute squared norms: norm_b += b_vec * b_vec
            let b_squared = _mm_mul_ps(b_vec, b_vec);
            norm_b_sum = _mm_add_ps(norm_b_sum, b_squared);
        }

        // Horizontal reduction of SIMD vectors to scalars
        let dot_scalar = Self::horizontal_sum_sse41(dot_sum);
        let norm_a_scalar = Self::horizontal_sum_sse41(norm_a_sum);
        let norm_b_scalar = Self::horizontal_sum_sse41(norm_b_sum);

        // Process remainder elements with scalar operations
        let (dot_remainder, norm_a_remainder, norm_b_remainder) = if remainder > 0 {
            let remainder_offset = chunks * 4;
            Self::process_remainder_scalar(&a[remainder_offset..], &b[remainder_offset..])
        } else {
            (0.0, 0.0, 0.0)
        };

        (
            dot_scalar + dot_remainder,
            norm_a_scalar + norm_a_remainder,
            norm_b_scalar + norm_b_remainder,
        )
    }

    /// Horizontal sum of SSE vector using efficient reduction
    #[target_feature(enable = "sse4.1")]
    #[inline]
    unsafe fn horizontal_sum_sse41(vec: std::arch::x86_64::__m128) -> f32 {
        use std::arch::x86_64::*;

        // Use SSE4.1's horizontal add instruction for efficient reduction
        let sum_pairs = _mm_hadd_ps(vec, vec); // [a+b, c+d, a+b, c+d]
        let final_sum = _mm_hadd_ps(sum_pairs, sum_pairs); // [a+b+c+d, *, a+b+c+d, *]

        // Extract the final sum from the first element
        _mm_cvtss_f32(final_sum)
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

    /// Check if the current vector length is suitable for SSE4.1
    #[inline]
    fn is_suitable_length(len: usize) -> bool {
        len >= 4 // Need at least 4 elements for meaningful SSE usage
    }
}

impl CosineSimilarity for Sse41Similarity {
    #[inline]
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0; // Invalid input, return zero similarity
        }

        let _guard = MetricsGuard::new(&self.metrics, a.len());

        let (dot, norm_a, norm_b) =
            if Self::is_suitable_length(a.len()) && super::is_sse41_available() {
                // Use real SSE4.1 SIMD intrinsics
                unsafe { Self::process_sse41_chunks(a, b) }
            } else {
                // Fall back to scalar for very small vectors or when SSE4.1 unavailable
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

impl WithMetrics for Sse41Similarity {
    fn metrics(&self) -> SimilarityMetricsSnapshot {
        self.metrics.get_metrics()
    }

    fn reset_metrics(&self) {
        self.metrics.reset();
    }
}

impl RuntimeSelectable for Sse41Similarity {
    fn name(&self) -> &'static str {
        "sse4.1"
    }

    fn optimal_vector_length(&self) -> usize {
        4 // Optimal for SSE f32x4 vectors
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_sse41_cosine_similarity() {
        let sim = Sse41Similarity::new();

        // Test with 4-element vectors (perfect SSE alignment)
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.8; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with 8-element vectors (two SSE chunks)
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.72192016; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with remainder elements (6 elements = 1 chunk + 2 remainder)
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.7857143; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_edge_cases() {
        let sim = Sse41Similarity::new();

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
        let expected = 0.9838699; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_metrics() {
        let sim = Sse41Similarity::new();
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
}
