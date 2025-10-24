//! ARM AArch64 SIMD optimizations

pub mod neon;

pub use neon::{NeonSimilarity, is_neon_available};
