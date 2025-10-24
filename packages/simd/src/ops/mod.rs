//! SIMD-optimized operations for machine learning workloads
//!
//! High-performance implementations of common ML operations with runtime CPU
//! feature detection and optimal SIMD utilization.

pub mod argmax;
pub mod softmax;
pub mod temperature;

// Re-export main operation functions for convenient access
pub use argmax::argmax;
pub use softmax::softmax;
pub use temperature::scale_temperature;
