//! Scalar (non-SIMD) implementations of similarity operations

use std::sync::Arc;

use super::traits::{CosineSimilarity, RuntimeSelectable, WithMetrics};
use crate::similarity::metrics::{MetricsGuard, SimilarityMetrics, SimilarityMetricsSnapshot};

/// Scalar implementation of cosine similarity
pub struct ScalarSimilarity {
    metrics: Arc<SimilarityMetrics>,
}

impl Default for ScalarSimilarity {
    fn default() -> Self {
        Self {
            metrics: Arc::new(SimilarityMetrics::default()),
        }
    }
}

impl ScalarSimilarity {
    /// Create a new scalar similarity instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute the dot product and squared norms for two vectors
    #[inline]
    #[must_use]
    pub fn dot_and_norms(a: &[f32], b: &[f32]) -> (f32, f32, f32) {
        let mut dot = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;

        for (&a_val, &b_val) in a.iter().zip(b) {
            dot += a_val * b_val;
            norm_a += a_val * a_val;
            norm_b += b_val * b_val;
        }

        (dot, norm_a, norm_b)
    }
}

impl CosineSimilarity for ScalarSimilarity {
    #[inline]
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0; // Invalid input, return zero similarity
        }

        let _guard = MetricsGuard::new(&self.metrics, a.len());

        let (dot, norm_a, norm_b) = Self::dot_and_norms(a, b);

        let norm_product = (norm_a * norm_b).sqrt();
        if norm_product <= f32::EPSILON {
            0.0
        } else {
            (dot / norm_product).clamp(-1.0, 1.0)
        }
    }
}

impl WithMetrics for ScalarSimilarity {
    fn metrics(&self) -> SimilarityMetricsSnapshot {
        self.metrics.get_metrics()
    }

    fn reset_metrics(&self) {
        self.metrics.reset();
    }
}

impl RuntimeSelectable for ScalarSimilarity {
    fn name(&self) -> &'static str {
        "scalar"
    }

    fn optimal_vector_length(&self) -> usize {
        0 // Always available, but not optimal for any length
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_scalar_cosine_similarity() {
        let sim = ScalarSimilarity::new();

        // Test with simple vectors
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.974_631_8; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with orthogonal vectors
        let a = [1.0, 0.0];
        let b = [0.0, 1.0];
        assert_relative_eq!(sim.cosine_similarity(&a, &b), 0.0);

        // Test with identical vectors
        let a = [1.0, 2.0, 3.0];
        assert_relative_eq!(sim.cosine_similarity(&a, &a), 1.0);
    }

    #[test]
    fn test_metrics() {
        let sim = ScalarSimilarity::new();
        sim.reset_metrics();

        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];

        // First call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 1);
        assert_eq!(metrics.total_elements_processed, 3);

        // Second call
        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 2);
        assert_eq!(metrics.total_elements_processed, 6);

        // Reset and verify
        sim.reset_metrics();
        let metrics = sim.metrics();
        assert_eq!(metrics.total_calculations, 0);
        assert_eq!(metrics.total_elements_processed, 0);
    }
}
