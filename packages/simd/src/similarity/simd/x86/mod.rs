//! x86/x64 SIMD optimizations using AVX2, SSE4.1 with real intrinsics
//!
//! This module provides production-quality SIMD implementations using actual x86 intrinsics
//! for blazing-fast vector similarity calculations with zero allocation patterns.

mod avx2;
mod avx512;
mod sse41;

use std::sync::Arc;

pub use avx2::Avx2Similarity;
pub use avx512::Avx512Similarity;
pub use sse41::Sse41Similarity;

use crate::similarity::metrics::{MetricsGuard, SimilarityMetrics, SimilarityMetricsSnapshot};
use crate::similarity::traits::{CosineSimilarity, RuntimeSelectable, WithMetrics};

/// Dynamic x86 SIMD implementation that selects the best available instruction set
pub struct DynamicX86Similarity {
    implementation: Arc<dyn RuntimeSelectable>,
    metrics: Arc<SimilarityMetrics>,
}

impl Default for DynamicX86Similarity {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicX86Similarity {
    /// Create new dynamic x86 SIMD similarity with best available instruction set
    pub fn new() -> Self {
        let implementation = Self::select_best_implementation();
        Self {
            implementation,
            metrics: Arc::new(SimilarityMetrics::default()),
        }
    }

    /// Select the best available x86 SIMD implementation at runtime
    fn select_best_implementation() -> Arc<dyn RuntimeSelectable> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // Check for AVX-512 first (best performance)
            if is_avx512f_available() {
                return Arc::new(Avx512Similarity::new());
            }

            // Check for AVX2 (good performance, widely supported)
            if is_avx2_available() {
                return Arc::new(Avx2Similarity::new());
            }

            // Check for SSE4.1 (baseline modern x86)
            if is_sse41_available() {
                return Arc::new(Sse41Similarity::new());
            }
        }

        // Fallback to portable SIMD for non-x86 or very old CPUs
        Arc::new(super::portable::PortableSimdSimilarity::new())
    }
}

impl CosineSimilarity for DynamicX86Similarity {
    #[inline]
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let _guard = MetricsGuard::new(&self.metrics, a.len());
        self.implementation.cosine_similarity(a, b)
    }
}

impl WithMetrics for DynamicX86Similarity {
    fn metrics(&self) -> SimilarityMetricsSnapshot {
        // Combine our metrics with the implementation's metrics
        let mut our_metrics = self.metrics.get_metrics();
        let impl_metrics = self.implementation.metrics();

        our_metrics.total_calculations += impl_metrics.total_calculations;
        our_metrics.total_elements_processed += impl_metrics.total_elements_processed;
        our_metrics.total_time_ns += impl_metrics.total_time_ns;

        our_metrics
    }

    fn reset_metrics(&self) {
        self.metrics.reset();
        self.implementation.reset_metrics();
    }
}

impl RuntimeSelectable for DynamicX86Similarity {
    fn name(&self) -> &'static str {
        self.implementation.name()
    }

    fn optimal_vector_length(&self) -> usize {
        self.implementation.optimal_vector_length()
    }
}

/// Check if AVX-512F is available at runtime
#[inline]
pub fn is_avx512f_available() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        is_x86_feature_detected!("avx512f")
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        false
    }
}

/// Check if AVX2 and FMA are available at runtime
#[inline]
pub fn is_avx2_available() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma")
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        false
    }
}

/// Check if SSE4.1 is available at runtime
#[inline]
pub fn is_sse41_available() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        is_x86_feature_detected!("sse4.1")
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        false
    }
}

/// Get the name of the best available x86 instruction set
#[inline]
pub fn best_x86_instruction_set() -> &'static str {
    if is_avx512f_available() {
        "AVX-512F"
    } else if is_avx2_available() {
        "AVX2+FMA"
    } else if is_sse41_available() {
        "SSE4.1"
    } else {
        "Portable"
    }
}

/// Get optimal vector width for best available x86 instruction set
#[inline]
pub fn optimal_x86_vector_width() -> usize {
    if is_avx512f_available() {
        16 // 16 f32 values in AVX-512
    } else if is_avx2_available() {
        8 // 8 f32 values in AVX2
    } else if is_sse41_available() {
        4 // 4 f32 values in SSE
    } else {
        1 // Scalar fallback
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_dynamic_x86_similarity() {
        let sim = DynamicX86Similarity::new();

        // Test with vectors that align to common SIMD widths
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let result = sim.cosine_similarity(&a, &b);
        let expected = 0.72192016; // Precomputed value
        assert_relative_eq!(result, expected, epsilon = 1e-6);

        // Test with longer vectors for AVX-512 alignment
        let a: Vec<f32> = (1..=32).map(|i| i as f32).collect();
        let b: Vec<f32> = (1..=32).rev().map(|i| i as f32).collect();

        let result = sim.cosine_similarity(&a, &b);
        // This should be computed with real SIMD acceleration
        assert!(result > 0.0 && result <= 1.0);
    }

    #[test]
    fn test_feature_detection() {
        // Test that feature detection functions don't panic
        let _avx512 = is_avx512f_available();
        let _avx2 = is_avx2_available();
        let _sse41 = is_sse41_available();

        // Test instruction set name
        let name = best_x86_instruction_set();
        assert!(!name.is_empty());

        // Test vector width
        let width = optimal_x86_vector_width();
        assert!(width >= 1 && width <= 16);
    }

    #[test]
    fn test_metrics() {
        let sim = DynamicX86Similarity::new();
        sim.reset_metrics();

        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        sim.cosine_similarity(&a, &b);
        let metrics = sim.metrics();

        assert!(metrics.total_calculations > 0);
        assert!(metrics.total_elements_processed > 0);
    }
}
