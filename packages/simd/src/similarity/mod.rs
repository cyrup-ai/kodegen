//! High-performance vector similarity operations with automatic SIMD acceleration
//!
//! This module provides optimized implementations of common similarity metrics
//! with automatic selection of the best available implementation at runtime.

pub mod metrics;
mod scalar;
mod simd;
mod traits;

use std::sync::Arc;

use lazy_static::lazy_static;
pub use metrics::SimilarityMetricsSnapshot;
pub use scalar::ScalarSimilarity;
pub use simd::portable::PortableSimdSimilarity;
pub use traits::{CosineSimilarity, RuntimeSelectable, SimilarityBuilder};

lazy_static! {
    /// Global instance of the best available similarity implementation
    static ref GLOBAL_SIMILARITY: Arc<dyn RuntimeSelectable> = simd::best_available();
}

/// Compute the cosine similarity between two vectors using the best available implementation
///
/// This function automatically selects the optimal implementation based on:
/// - Vector length
/// - CPU features available at runtime
/// - Cache line alignment
///
/// # Panics
/// - If input vectors have different lengths
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(
        a.len(),
        b.len(),
        "Vectors must have the same length for cosine similarity"
    );

    // Always use the global implementation for consistent metrics tracking
    GLOBAL_SIMILARITY.cosine_similarity(a, b)
}

/// Smart cosine similarity that automatically selects the best implementation
///
/// This is an alias for [`cosine_similarity`] that provides automatic
/// runtime selection of the optimal implementation. The name "smart" indicates
/// the intelligent selection of SIMD vs scalar implementations based on
/// vector size and CPU capabilities.
#[inline]
pub fn smart_cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    cosine_similarity(a, b)
}

/// SIMD-optimized cosine similarity (alias for compatibility)
///
/// This function is an alias for the general [`cosine_similarity`] function,
/// which already uses SIMD when appropriate. Kept for API compatibility.
#[inline]
pub fn simd_cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    cosine_similarity(a, b)
}

/// Get the name of the currently active similarity implementation
#[inline]
pub fn active_implementation() -> &'static str {
    GLOBAL_SIMILARITY.name()
}

/// Get metrics about similarity operations
#[inline]
pub fn metrics() -> SimilarityMetricsSnapshot {
    GLOBAL_SIMILARITY.metrics()
}

/// Reset all similarity metrics to zero
#[inline]
pub fn reset_metrics() {
    GLOBAL_SIMILARITY.reset_metrics();
}

/// Create a new similarity builder for custom configuration
#[inline]
pub fn builder() -> SimilarityBuilder {
    SimilarityBuilder::new()
}
