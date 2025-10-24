//! Memory retrieval strategies and algorithms
//!
//! This module provides multiple strategies for retrieving relevant memories:
//! - **Semantic**: Vector similarity using embeddings
//! - **Temporal**: Time-based with exponential decay
//! - **Hybrid**: Combines multiple strategies with cognitive filtering
//! - **Manager**: Orchestrates strategies with unified API

// Submodules
pub mod hybrid;
pub mod manager;
pub mod semantic;
pub mod strategy;
pub mod temporal;
pub mod types;

// Re-export all public types to maintain API compatibility
pub use hybrid::HybridRetrieval;
pub use manager::RetrievalManager;
pub use semantic::SemanticRetrieval;
pub use strategy::RetrievalStrategy;
pub use temporal::TemporalRetrieval;
pub use types::{PendingRetrieval, RetrievalMethod, RetrievalResult};
