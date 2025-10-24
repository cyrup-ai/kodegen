//! Event type definitions for the crawl event system
//!
//! This module contains the core event types and metadata structures used
//! throughout the crawling process.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Reason for event bus shutdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShutdownReason {
    /// Crawl completed successfully
    CrawlCompleted,
    /// Crawl encountered an error
    Error(String),
    /// Crawl was cancelled by user
    Cancelled,
}

/// Event types emitted during the crawl process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrawlEvent {
    /// Emitted when a crawl session starts
    CrawlStarted {
        start_url: String,
        output_dir: PathBuf,
        max_depth: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Emitted when a page has been successfully crawled and saved
    PageCrawled {
        url: String,
        local_path: PathBuf,
        depth: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
        metadata: PageCrawlMetadata,
    },
    /// Emitted when link rewriting has been completed for a page
    LinkRewriteCompleted {
        target_url: String,
        files_updated: usize,
        links_rewritten: usize,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Emitted when the entire crawl session completes
    CrawlCompleted {
        total_pages: usize,
        total_links_rewritten: usize,
        duration: std::time::Duration,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Cache hit - page skipped because etag matched
    CacheHit {
        url: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Signals that the event bus is shutting down
    ///
    /// Subscribers should exit their event loops when receiving this event.
    Shutdown {
        reason: ShutdownReason,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// Metadata about a crawled page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageCrawlMetadata {
    /// Size of the original HTML content in bytes
    pub html_size: usize,
    /// Size of the saved compressed file in bytes
    pub compressed_size: usize,
    /// Number of links found on the page
    pub links_found: usize,
    /// Number of links filtered for crawling
    pub links_for_crawling: usize,
    /// Whether screenshot was captured successfully
    pub screenshot_captured: bool,
    /// Time taken to process the page
    pub processing_duration: std::time::Duration,
}

/// Result of publishing a batch of events
///
/// Provides detailed information about batch publication success/failure.
/// Unlike a Result type, this always represents successful execution of the
/// batch operation itself - the fields indicate how many individual events
/// succeeded or failed within the batch.
///
/// # Best-Effort Semantics
///
/// The event bus uses best-effort delivery. All events in the batch are attempted
/// regardless of individual failures. This struct transparently reports what happened
/// so callers can make informed decisions about partial success scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPublishResult {
    /// Total number of events in the batch
    pub total: usize,

    /// Number of events successfully published
    pub published: usize,

    /// Number of events that failed to publish (no active subscribers)
    pub failed: usize,

    /// Peak subscriber count observed during batch
    pub max_subscribers: usize,
}

impl BatchPublishResult {
    /// Check if all events were successfully published
    ///
    /// Returns true only if published == total and failed == 0
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.published == self.total && self.failed == 0
    }

    /// Check if any events failed to publish
    ///
    /// Returns true if failed > 0
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }

    /// Calculate success rate as a percentage
    ///
    /// Returns 100.0 if total is 0 (empty batch), otherwise (published / total) * 100.0
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.published as f64 / self.total as f64) * 100.0
    }
}

/// Helper functions for creating common events
impl CrawlEvent {
    /// Create a `CrawlStarted` event
    #[must_use]
    pub fn crawl_started(start_url: String, output_dir: PathBuf, max_depth: u32) -> Self {
        Self::CrawlStarted {
            start_url,
            output_dir,
            max_depth,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a `PageCrawled` event
    #[must_use]
    pub fn page_crawled(
        url: String,
        local_path: PathBuf,
        depth: u32,
        metadata: PageCrawlMetadata,
    ) -> Self {
        Self::PageCrawled {
            url,
            local_path,
            depth,
            timestamp: chrono::Utc::now(),
            metadata,
        }
    }

    /// Create a `LinkRewriteCompleted` event
    #[must_use]
    pub fn link_rewrite_completed(
        target_url: String,
        files_updated: usize,
        links_rewritten: usize,
    ) -> Self {
        Self::LinkRewriteCompleted {
            target_url,
            files_updated,
            links_rewritten,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a `CrawlCompleted` event
    #[must_use]
    pub fn crawl_completed(
        total_pages: usize,
        total_links_rewritten: usize,
        duration: std::time::Duration,
    ) -> Self {
        Self::CrawlCompleted {
            total_pages,
            total_links_rewritten,
            duration,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a cache hit event
    #[must_use]
    pub fn cache_hit(url: String) -> Self {
        Self::CacheHit {
            url,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a Shutdown event
    #[must_use]
    pub fn shutdown(reason: ShutdownReason) -> Self {
        Self::Shutdown {
            reason,
            timestamp: chrono::Utc::now(),
        }
    }
}
