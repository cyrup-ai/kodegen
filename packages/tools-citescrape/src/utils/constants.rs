//! Shared configuration constants for citescrape
//!
//! This module contains default values and configuration constants used
//! throughout the codebase to ensure consistency and avoid magic numbers.

/// Default crawl rate: 2 requests per second
///
/// Conservative rate that respects server resources while maintaining
/// reasonable crawl speed. Most servers can handle this rate without issue.
///
/// Users can adjust via `crawl_rate_rps` parameter:
/// - Increase for fast servers or local testing
/// - Decrease for slow servers or rate-limited APIs
pub const DEFAULT_CRAWL_RATE_RPS: f64 = 2.0;

/// Screenshot quality: 80% JPEG compression
///
/// Balances file size (~50-100KB per screenshot) with visual quality.
/// Higher values increase storage requirements significantly:
/// - 90: ~150KB (2x size, minimal quality improvement)
/// - 70: ~30KB (acceptable for thumbnails)
/// - 80: Sweet spot for documentation capture
pub const SCREENSHOT_QUALITY: u8 = 80;

/// Search batch size: 1000 documents
///
/// Number of documents to process in a single batch during search indexing.
/// Tuned for balance between memory usage (~10-50MB per batch) and throughput.
///
/// Increasing this improves throughput but increases memory usage.
/// Decreasing this reduces memory usage but may slow indexing.
pub const SEARCH_BATCH_SIZE: usize = 1000;

/// Default maximum crawl depth: 3 levels
///
/// Limits how deep the crawler will follow links from the starting URL.
/// Helps prevent unbounded crawling while capturing most relevant content.
pub const DEFAULT_MAX_DEPTH: u8 = 3;
