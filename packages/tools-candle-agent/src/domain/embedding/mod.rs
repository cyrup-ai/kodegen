//! Embedding System Module
//!
//! This module provides a consolidated embedding system with sparse file separation
//! for singular concerns. Each submodule handles a specific aspect of embedding functionality.

/// Embedding configuration types and builder patterns
pub mod config;

/// Usage tracking and token counting for embedding operations
pub mod usage;

/// Embedding service implementations and caching mechanisms
pub mod service;

/// Similarity computation types and traits for vector comparisons
pub mod similarity;

pub use config::EmbeddingConfig as EmbeddingConfiguration;
// Re-export configuration types
pub use config::{EmbeddingConfig, IntoEmbeddingConfig};
// Re-export service types
pub use service::{EmbeddingPool, EmbeddingService, InMemoryEmbeddingCache};
// Re-export similarity types
pub use similarity::{Similarity, SimilarityMetric, SimilarityResult};
// Re-export usage tracking types
pub use usage::{EmbeddingUsage, TokenUsage};
