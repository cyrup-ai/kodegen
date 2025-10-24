//! Ultra-High-Performance SIMD Operations for cyrup Ecosystem
//!
//! Production-quality vectorized implementations shared across cyrup packages:
//! - Vector similarity operations (from memory package)
//! - Platform-specific optimizations and fallbacks
//!
//! ## Core Features
//!
//! - **Vectorized Similarity**: Parallel cosine similarity with runtime CPU feature detection
//! - **Zero Allocation**: Pre-allocated buffers and stack-based temporary storage
//! - **Adaptive Selection**: Automatic SIMD vs scalar selection based on vector size
//! - **Platform Support**: `x86_64` `AVX2`, `ARM64` `NEON` with portable fallbacks
//!
//! ## Usage Examples
//!
//! **Similarity Operations:**
//! ```rust,no_run
//! use kodegen_simd::similarity::smart_cosine_similarity;
//!
//! let a = vec![1.0, 2.0, 3.0, 4.0];
//! let b = vec![4.0, 3.0, 2.0, 1.0];
//! let similarity = smart_cosine_similarity(&a, &b);
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Public modules
pub mod benchmark;
pub mod config;
pub mod constants;
pub mod context;
pub mod error;
pub mod logits;
pub mod ops;
/// Runtime CPU feature detection and dispatch for SIMD operations
pub mod runtime;
/// Serde type constraints for structured generation
pub mod serde_constraints;
pub mod similarity;
pub mod utils;

// Re-export core types for ergonomic usage
// Re-export constants
pub use constants::{SIMD_WIDTH_8, VERSION};
pub use error::{SimdError, SimdResult};
// Re-export logits operations
pub use logits::{apply_penalties_simd, prepare_nucleus_sampling_simd, topk_filtering_simd};
// Re-export ops (temperature and softmax operations)
pub use ops::{argmax, scale_temperature, softmax};
// Re-export runtime CPU detection
pub use runtime::{CpuFeatures, CpuInfo, get_cpu_features, get_cpu_info, should_use_simd};
pub use similarity::{cosine_similarity, simd_cosine_similarity, smart_cosine_similarity};
pub use utils::simd_available;
