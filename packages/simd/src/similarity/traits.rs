//! Core traits for similarity operations

use std::sync::Arc;

use crate::similarity::metrics::SimilarityMetricsSnapshot;

/// A trait for types that can compute cosine similarity between vectors
pub trait CosineSimilarity {
    /// Compute the cosine similarity between two vectors with bounds checking
    ///
    /// # Panics
    /// If the input vectors have different lengths
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32;
}

/// A trait for similarity operations that can provide metrics
pub trait WithMetrics {
    /// Get a snapshot of the current metrics
    fn metrics(&self) -> SimilarityMetricsSnapshot;

    /// Reset all metrics to zero
    fn reset_metrics(&self);
}

/// A trait for implementations that can be selected at runtime
pub trait RuntimeSelectable: CosineSimilarity + WithMetrics + Send + Sync {
    /// Get the name of this implementation (e.g., "scalar", "avx2", "neon")
    fn name(&self) -> &'static str;

    /// The minimum vector length for which this implementation is optimal
    fn optimal_vector_length(&self) -> usize;
}

/// A builder for similarity operations
pub struct SimilarityBuilder {
    prefer_simd: bool,
    min_simd_elements: usize,
}

impl Default for SimilarityBuilder {
    fn default() -> Self {
        Self {
            prefer_simd: true,
            min_simd_elements: 16, // Default threshold for SIMD
        }
    }
}

impl SimilarityBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable SIMD acceleration
    pub fn with_simd(mut self, enable: bool) -> Self {
        self.prefer_simd = enable;
        self
    }

    /// Set the minimum number of elements for SIMD acceleration
    pub fn with_min_simd_elements(mut self, min_elements: usize) -> Self {
        self.min_simd_elements = min_elements;
        self
    }

    /// Build a runtime-selected similarity implementation
    pub fn build(self) -> Arc<dyn RuntimeSelectable> {
        // Return the best available implementation based on current features
        crate::similarity::simd::best_available()
    }
}
