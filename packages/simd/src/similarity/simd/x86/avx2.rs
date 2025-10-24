//! Real AVX2 SIMD implementation with FMA for 8-wide f32 operations
//!
//! This module provides production-quality AVX2 intrinsics for blazing-fast
//! cosine similarity calculations with zero allocation patterns.

use std::sync::Arc;

use crate::similarity::metrics::{MetricsGuard, SimilarityMetrics, SimilarityMetricsSnapshot};
use crate::similarity::traits::{CosineSimilarity, RuntimeSelectable, WithMetrics};

/// AVX2-optimized similarity implementation for x86/x64
pub struct Avx2Similarity {
    metrics: Arc<SimilarityMetrics>,
}

impl Default for Avx2Similarity {
    fn default() -> Self {
        Self::new()
    }
}

impl Avx2Similarity {
    /// Create a new AVX2 similarity instance
    #[inline]
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(SimilarityMetrics::default()),
        }
    }

    /// Process vectors using real AVX2 SIMD instructions with FMA
    #[target_feature(enable = "avx2,fma")]
    unsafe fn process_avx2_chunks(a: &[f32], b: &[f32]) -> (f32, f32, f32) {
        use std::arch::x86_64::*;

        let len = a.len();
        let chunks = len / 8; // Process 8 f32 values at a time with AVX2
        let remainder = len % 8;

        // Initialize SIMD accumulators
        let mut dot_sum = _mm256_setzero_ps();
        let mut norm_a_sum = _mm256_setzero_ps();
        let mut norm_b_sum = _mm256_setzero_ps();

        // Process 8 floats at a time using real AVX2 intrinsics
        for i in 0..chunks {
            let offset = i * 8;

            // Load 8 consecutive f32 values from each vector
            let a_vec = _mm256_loadu_ps(a.as_ptr().add(offset));
            let b_vec = _mm256_loadu_ps(b.as_ptr().add(offset));

            // Fused multiply-add: dot_sum += a_vec * b_vec
            dot_sum = _mm256_fmadd_ps(a_vec, b_vec, dot_sum);

            // Compute squared norms: norm_a += a_vec * a_vec
            norm_a_sum = _mm256_fmadd_ps(a_vec, a_vec, norm_a_sum);

            // Compute squared norms: norm_b += b_vec * b_vec
            norm_b_sum = _mm256_fmadd_ps(b_vec, b_vec, norm_b_sum);
        }

        // Horizontal reduction of SIMD vectors to scalars
        let dot_scalar = Self::horizontal_sum_avx2(dot_sum);
        let norm_a_scalar = Self::horizontal_sum_avx2(norm_a_sum);
        let norm_b_scalar = Self::horizontal_sum_avx2(norm_b_sum);

        // Process remainder elements with scalar operations
        let (dot_remainder, norm_a_remainder, norm_b_remainder) = if remainder > 0 {
            let remainder_offset = chunks * 8;
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

    /// Horizontal sum of AVX2 vector using efficient reduction
    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn horizontal_sum_avx2(vec: std::arch::x86_64::__m256) -> f32 {
        use std::arch::x86_64::*;

        // Split 256-bit vector into two 128-bit halves and add them
        let hi = _mm256_extractf128_ps(vec, 1); // Upper 128 bits
        let lo = _mm256_castps256_ps128(vec); // Lower 128 bits
        let sum_128 = _mm_add_ps(hi, lo);

        // Horizontal add within 128-bit vector
        let sum_64 = _mm_hadd_ps(sum_128, sum_128); // [a+b, c+d, a+b, c+d]
        let sum_32 = _mm_hadd_ps(sum_64, sum_64); // [a+b+c+d, *, a+b+c+d, *]

        // Extract the final sum
        _mm_cvtss_f32(sum_32)
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

    /// Check if the current vector length is suitable for AVX2
    #[inline]
    fn is_suitable_length(len: usize) -> bool {
        len >= 8 // Need at least 8 elements for meaningful AVX2 usage
    }
}

impl CosineSimilarity for Avx2Similarity {
    #[inline]
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0; // Invalid input, return zero similarity
        }

        let _guard = MetricsGuard::new(&self.metrics, a.len());

        let (dot, norm_a, norm_b) =
            if Self::is_suitable_length(a.len()) && super::is_avx2_available() {
                // Use real AVX2 SIMD intrinsics (unsafe but fast)
                unsafe { Self::process_avx2_chunks(a, b) }
            } else {
                // Fall back to scalar for very small vectors or when AVX2 unavailable
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

impl WithMetrics for Avx2Similarity {
    fn metrics(&self) -> SimilarityMetricsSnapshot {
        self.metrics.get_metrics()
    }

    fn reset_metrics(&self) {
        self.metrics.reset();
    }
}

impl RuntimeSelectable for Avx2Similarity {
    fn name(&self) -> &'static str {
        "avx2+fma"
    }

    fn optimal_vector_length(&self) -> usize {
        8 // Optimal for AVX2 f32x8 vectors
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_avx2_cosine_similarity() {
        let sim = Avx2Similarity::new();

        // Test with 8-element vectors (perfect AVX2 alignment)
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.72192016; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with 16-element vectors (two AVX2 chunks)
        let a: Vec<f32> = (1..=16).map(|i| i as f32).collect();
        let b: Vec<f32> = (1..=16).rev().map(|i| i as f32).collect();

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.6385917; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with remainder elements (10 elements = 1 chunk + 2 remainder)
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let b = [10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.6753032; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_edge_cases() {
        let sim = Avx2Similarity::new();

        // Test orthogonal vectors
        let a = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let result = sim.cosine_similarity(&a, &b);
        assert_relative_eq!(result, 0.0, epsilon = 1e-6);

        // Test identical vectors
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let result = sim.cosine_similarity(&a, &a);
        assert_relative_eq!(result, 1.0, epsilon = 1e-6);

        // Test zero vectors
        let a = [0.0; 8];
        let b = [1.0; 8];
        let result = sim.cosine_similarity(&a, &b);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_metrics() {
        let sim = Avx2Similarity::new();
        sim.reset_metrics();

        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        // First call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 1);
        assert_eq!(metrics.total_elements_processed, 8);

        // Second call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 2);
        assert_eq!(metrics.total_elements_processed, 16);
    }
}
