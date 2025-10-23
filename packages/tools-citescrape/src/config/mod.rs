//! Configuration module for web crawling
//!
//! This module provides the `CrawlConfig` struct and its type-safe builder
//! for configuring web crawling operations with validation and sensible defaults.

// Sub-modules
pub mod builder;
pub mod getters;
pub mod methods;
pub mod types;

// Re-exports for public API
pub use builder::{Complete, CrawlConfigBuilder, WithStartUrl, WithStorageDir};
pub use types::CrawlConfig;
