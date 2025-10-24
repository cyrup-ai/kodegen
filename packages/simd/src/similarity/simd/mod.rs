//! SIMD-accelerated similarity operations with real hardware intrinsics
//!
//! This module contains production-quality platform-specific SIMD implementations.
//! All implementations use actual SIMD intrinsics for maximum performance.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

pub mod portable;

use std::sync::Arc;

use lazy_static::lazy_static;

use super::traits::RuntimeSelectable;

/// Get the best available REAL SIMD implementation for the current CPU
pub fn best_available() -> Arc<dyn RuntimeSelectable> {
    lazy_static! {
        static ref BEST_IMPL: Arc<dyn RuntimeSelectable> = {
            // Check for platform-specific REAL SIMD features first
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            {
                // Try AVX-512F first (best performance)
                if x86::is_avx512f_available() {
                    return Arc::new(x86::Avx512Similarity::new());
                }

                // Try AVX2+FMA (excellent performance)
                if x86::is_avx2_available() {
                    return Arc::new(x86::Avx2Similarity::new());
                }

                // Try SSE4.1 (good performance, broad compatibility)
                if x86::is_sse41_available() {
                    return Arc::new(x86::Sse41Similarity::new());
                }

                // Use dynamic x86 selector as fallback
                return Arc::new(x86::DynamicX86Similarity::new());
            }

            #[cfg(target_arch = "aarch64")]
            {
                if aarch64::is_neon_available() {
                    return Arc::new(aarch64::NeonSimilarity::new());
                }
            }

            // Fall back to portable SIMD (still real SIMD using `wide` crate)
            Arc::new(portable::PortableSimdSimilarity::new())
        };
    }

    BEST_IMPL.clone()
}
