//! Document builder implementations - Zero Box<dyn> trait-based architecture
//!
//! This module provides builders for loading documents from various sources:
//! - Local files
//! - URLs  
//! - GitHub repositories
//! - Glob patterns
//! - Direct text/data
//!
//! All loading operations support streaming, chunking, error handling, and retry logic.

mod api;
mod builder_impl;
mod detection;
mod loaders;
mod trait_def;
mod types;

// Re-export public API
pub use api::document;
pub use trait_def::DocumentBuilder;

// Note: The impl Document blocks in api.rs are automatically available
// when this module is used, extending the Document domain type
