//! Portable SIMD implementation using the `wide` crate for cross-platform support

use std::sync::Arc;

use wide::f32x8;

use super::super::metrics::{MetricsGuard, SimilarityMetrics, SimilarityMetricsSnapshot};
use crate::similarity::traits::{CosineSimilarity, RuntimeSelectable, WithMetrics};

/// Portable SIMD implementation using the `wide` crate
pub struct PortableSimdSimilarity {
    metrics: Arc<SimilarityMetrics>,
}

impl Default for PortableSimdSimilarity {
    fn default() -> Self {
        Self {
            metrics: Arc::new(SimilarityMetrics::default()),
        }
    }
}

impl PortableSimdSimilarity {
    /// Create a new portable SIMD similarity instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Process chunks of vectors using SIMD
    #[inline]
    fn process_chunks<const N: usize>(a: &[f32], b: &[f32]) -> (f32, f32, f32) {
        let chunks = a.chunks_exact(N);
        let remainder = chunks.remainder();
        let b_chunks = b.chunks_exact(N);

        let mut dot = f32x8::ZERO;
        let mut norm_a = f32x8::ZERO;
        let mut norm_b = f32x8::ZERO;

        // Process full chunks
        for (a_chunk, b_chunk) in chunks.zip(b_chunks) {
            let a_simd = f32x8::new([
                a_chunk[0], a_chunk[1], a_chunk[2], a_chunk[3], a_chunk[4], a_chunk[5], a_chunk[6],
                a_chunk[7],
            ]);

            let b_simd = f32x8::new([
                b_chunk[0], b_chunk[1], b_chunk[2], b_chunk[3], b_chunk[4], b_chunk[5], b_chunk[6],
                b_chunk[7],
            ]);

            dot = (a_simd * b_simd) + dot;
            norm_a = (a_simd * a_simd) + norm_a;
            norm_b = (b_simd * b_simd) + norm_b;
        }

        // Reduce SIMD vectors to scalars
        let dot_scalar = dot.reduce_add();
        let a_norm_scalar = norm_a.reduce_add();
        let b_norm_scalar = norm_b.reduce_add();

        // Process remainder
        let (dot_rem, a_norm_rem, b_norm_rem) = if !remainder.is_empty() {
            let a_remainder = &a[a.len() - remainder.len()..];
            let b_remainder = &b[b.len() - remainder.len()..];
            crate::similarity::ScalarSimilarity::dot_and_norms(a_remainder, b_remainder)
        } else {
            (0.0, 0.0, 0.0)
        };

        (
            dot_scalar + dot_rem,
            a_norm_scalar + a_norm_rem,
            b_norm_scalar + b_norm_rem,
        )
    }
}

impl CosineSimilarity for PortableSimdSimilarity {
    #[inline]
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0; // Invalid input, return zero similarity
        }

        let _guard = MetricsGuard::new(&self.metrics, a.len());

        let (dot, norm_a, norm_b) = if a.len() >= 8 {
            Self::process_chunks::<8>(a, b)
        } else {
            // Fall back to scalar for very small vectors
            crate::similarity::ScalarSimilarity::dot_and_norms(a, b)
        };

        let norm_product = (norm_a * norm_b).sqrt();
        if norm_product <= f32::EPSILON {
            0.0
        } else {
            (dot / norm_product).clamp(-1.0, 1.0)
        }
    }
}

impl WithMetrics for PortableSimdSimilarity {
    fn metrics(&self) -> SimilarityMetricsSnapshot {
        self.metrics.get_metrics()
    }

    fn reset_metrics(&self) {
        self.metrics.reset();
    }
}

impl RuntimeSelectable for PortableSimdSimilarity {
    fn name(&self) -> &'static str {
        "portable-simd"
    }

    fn optimal_vector_length(&self) -> usize {
        8 // Optimal for f32x8 vectors
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_portable_simd_cosine_similarity() {
        let sim = PortableSimdSimilarity::new();

        // Test with simple vectors
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.588_235_3; // Correct value: dot=120, norm=204, cosine=120/204=0.5882353
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with vectors that have a remainder
        let a = [1.0, 2.0, 3.0, 4.0, 5.0];
        let b = [5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.636_363_6; // Correct value: dot=35, norm=55, cosine=35/55=0.6363636
        assert_relative_eq!(result, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_metrics() {
        let sim = PortableSimdSimilarity::new();
        sim.reset_metrics();

        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        // First call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 1);
        assert_eq!(metrics.total_elements_processed, 8);

        // Second call with different length
        let a = [1.0, 2.0, 3.0];
        let b = [3.0, 2.0, 1.0];
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 2);
        assert_eq!(metrics.total_elements_processed, 11);
    }
}
