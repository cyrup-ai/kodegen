//! Event system for tracking and reporting crawl progress
//!
//! This module provides a comprehensive event bus system for publishing and
//! subscribing to crawl events, with support for metrics, filtering, and batching.

// Sub-modules
pub mod bus;
pub mod config;
pub mod errors;
pub mod metrics;
pub mod streaming;
pub mod types;

// Re-exports for public API
pub use bus::CrawlEventBus;
pub use config::EventBusConfig;
pub use errors::EventBusError;
pub use metrics::EventBusMetrics;
pub use streaming::FilteredReceiver;
pub use types::{BatchPublishResult, CrawlEvent, PageCrawlMetadata, ShutdownReason};
