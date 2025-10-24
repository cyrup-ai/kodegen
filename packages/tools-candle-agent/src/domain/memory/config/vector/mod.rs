//! Vector store configuration for embeddings and similarity search
//!
//! This module provides configuration types for vector stores with SIMD optimization,
//! multiple distance metrics, and various index types.
//!
//! ## Main Types
//!
//! - [`VectorStoreConfig`] - Primary configuration struct
//! - [`VectorStoreType`] - Supported vector store backends
//! - [`DistanceMetric`] - Similarity/distance calculation methods
//! - [`IndexType`] - Vector index structures
//! - [`SimdConfig`] - SIMD optimization settings
//! - [`IndexConfig`] - Index-specific parameters
//! - [`PerformanceConfig`] - Performance tuning options
//! - [`MemoryConfig`] - Memory usage tracking and limits

mod config;
mod index;
mod performance;
mod simd;
mod types;

// Re-export all public types
pub use config::VectorStoreConfig;
pub use index::IndexConfig;
pub use performance::{MemoryConfig, PerformanceConfig, VectorConnectionConfig};
pub use simd::{SimdConfig, SimdInstructionSet};
pub use types::{AllocationStrategy, DistanceMetric, IndexType, VectorStoreType};
