//! Vector search functionality - THREAD-SAFE SYNCHRONOUS OPERATIONS
//!
//! This module provides comprehensive vector search capabilities using:
//! - Synchronous vector similarity search with SIMD acceleration
//! - Thread-safe embedding generation and caching
//! - Hybrid search combining vector and keyword approaches
//! - Zero-allocation search result processing
//! - Advanced filtering and ranking algorithms

mod cognitive;
mod core;
mod helpers;
mod hybrid;
mod options;
mod types;

// Re-export public API (maintains backward compatibility)
pub use core::VectorSearch;
pub use hybrid::HybridSearch;
pub use options::SearchOptions;
pub use types::{KeywordSearchFn, RequestInfoCallback, SearchResult};
