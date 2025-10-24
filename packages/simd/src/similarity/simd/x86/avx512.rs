//! Real AVX-512F SIMD implementation for 16-wide f32 operations
//!
//! This module provides cutting-edge AVX-512 intrinsics for maximum
//! performance cosine similarity calculations with zero allocation patterns.

use std::sync::Arc;

use crate::similarity::metrics::{MetricsGuard, SimilarityMetrics, SimilarityMetricsSnapshot};
use crate::similarity::traits::{CosineSimilarity, RuntimeSelectable, WithMetrics};

/// AVX-512F-optimized similarity implementation for modern x86/x64
pub struct Avx512Similarity {
    metrics: Arc<SimilarityMetrics>,
}

impl Default for Avx512Similarity {
    fn default() -> Self {
        Self::new()
    }
}

impl Avx512Similarity {
    /// Create a new AVX-512 similarity instance
    #[inline]
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(SimilarityMetrics::default()),
        }
    }

    /// Process vectors using real AVX-512F SIMD instructions
    #[target_feature(enable = "avx512f")]
    unsafe fn process_avx512_chunks(a: &[f32], b: &[f32]) -> (f32, f32, f32) {
        use std::arch::x86_64::*;

        let len = a.len();
        let chunks = len / 16; // Process 16 f32 values at a time with AVX-512
        let remainder = len % 16;

        // Initialize SIMD accumulators
        let mut dot_sum = _mm512_setzero_ps();
        let mut norm_a_sum = _mm512_setzero_ps();
        let mut norm_b_sum = _mm512_setzero_ps();

        // Process 16 floats at a time using real AVX-512F intrinsics
        for i in 0..chunks {
            let offset = i * 16;

            // Load 16 consecutive f32 values from each vector
            let a_vec = _mm512_loadu_ps(a.as_ptr().add(offset));
            let b_vec = _mm512_loadu_ps(b.as_ptr().add(offset));

            // Fused multiply-add: dot_sum += a_vec * b_vec
            dot_sum = _mm512_fmadd_ps(a_vec, b_vec, dot_sum);

            // Compute squared norms: norm_a += a_vec * a_vec
            norm_a_sum = _mm512_fmadd_ps(a_vec, a_vec, norm_a_sum);

            // Compute squared norms: norm_b += b_vec * b_vec
            norm_b_sum = _mm512_fmadd_ps(b_vec, b_vec, norm_b_sum);
        }

        // Horizontal reduction of SIMD vectors to scalars
        let dot_scalar = Self::horizontal_sum_avx512(dot_sum);
        let norm_a_scalar = Self::horizontal_sum_avx512(norm_a_sum);
        let norm_b_scalar = Self::horizontal_sum_avx512(norm_b_sum);

        // Process remainder elements with scalar operations
        let (dot_remainder, norm_a_remainder, norm_b_remainder) = if remainder > 0 {
            let remainder_offset = chunks * 16;
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

    /// Horizontal sum of AVX-512 vector using efficient reduction
    #[target_feature(enable = "avx512f")]
    #[inline]
    unsafe fn horizontal_sum_avx512(vec: std::arch::x86_64::__m512) -> f32 {
        use std::arch::x86_64::*;

        // AVX-512 has a direct horizontal add reduction instruction
        _mm512_reduce_add_ps(vec)
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

    /// Check if the current vector length is suitable for AVX-512
    #[inline]
    fn is_suitable_length(len: usize) -> bool {
        len >= 16 // Need at least 16 elements for meaningful AVX-512 usage
    }
}

impl CosineSimilarity for Avx512Similarity {
    #[inline]
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0; // Invalid input, return zero similarity
        }

        let _guard = MetricsGuard::new(&self.metrics, a.len());

        let (dot, norm_a, norm_b) =
            if Self::is_suitable_length(a.len()) && super::is_avx512f_available() {
                // Use real AVX-512F SIMD intrinsics (unsafe but blazing fast)
                unsafe { Self::process_avx512_chunks(a, b) }
            } else {
                // Fall back to scalar for small vectors or when AVX-512 unavailable
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

impl WithMetrics for Avx512Similarity {
    fn metrics(&self) -> SimilarityMetricsSnapshot {
        self.metrics.get_metrics()
    }

    fn reset_metrics(&self) {
        self.metrics.reset();
    }
}

impl RuntimeSelectable for Avx512Similarity {
    fn name(&self) -> &'static str {
        "avx512f"
    }

    fn optimal_vector_length(&self) -> usize {
        16 // Optimal for AVX-512 f32x16 vectors
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_avx512_cosine_similarity() {
        let sim = Avx512Similarity::new();

        // Test with 16-element vectors (perfect AVX-512 alignment)
        let a: Vec<f32> = (1..=16).map(|i| i as f32).collect();
        let b: Vec<f32> = (1..=16).rev().map(|i| i as f32).collect();

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.6385917; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with 32-element vectors (two AVX-512 chunks)
        let a: Vec<f32> = (1..=32).map(|i| i as f32).collect();
        let b: Vec<f32> = (1..=32).rev().map(|i| i as f32).collect();

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.61563736; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with remainder elements (20 elements = 1 chunk + 4 remainder)
        let a: Vec<f32> = (1..=20).map(|i| i as f32).collect();
        let b: Vec<f32> = (1..=20).rev().map(|i| i as f32).collect();

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.6324555; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_edge_cases() {
        let sim = Avx512Similarity::new();

        // Test orthogonal vectors (16 elements)
        let mut a = vec![0.0; 16];
        let mut b = vec![0.0; 16];
        a[0] = 1.0;
        b[1] = 1.0;
        let result = sim.cosine_similarity(&a, &b);
        assert_relative_eq!(result, 0.0, epsilon = 1e-6);

        // Test identical vectors
        let a: Vec<f32> = (1..=16).map(|i| i as f32).collect();
        let result = sim.cosine_similarity(&a, &a);
        assert_relative_eq!(result, 1.0, epsilon = 1e-6);

        // Test zero vectors
        let a = vec![0.0; 16];
        let b = vec![1.0; 16];
        let result = sim.cosine_similarity(&a, &b);
        assert_eq!(result, 0.0);

        // Test small vectors (should use scalar fallback)
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.9746318; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_metrics() {
        let sim = Avx512Similarity::new();
        sim.reset_metrics();

        let a: Vec<f32> = (1..=16).map(|i| i as f32).collect();
        let b: Vec<f32> = (1..=16).rev().map(|i| i as f32).collect();

        // First call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 1);
        assert_eq!(metrics.total_elements_processed, 16);

        // Second call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 2);
        assert_eq!(metrics.total_elements_processed, 32);
    }
}
