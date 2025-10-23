//! Core types and traits for web crawling operations.
//!
//! This module contains the fundamental types used throughout the crawler including
//! error types, progress indicators, and the main Crawler trait.

use anyhow::Result;
use std::fmt;

/// Custom error type for crawl operations
#[derive(Debug, Clone)]
pub enum CrawlError {
    /// Configuration error
    ConfigError(String),
    /// Browser error
    BrowserError(String),
    /// Network error
    NetworkError(String),
    /// Operation cancelled
    Cancelled,
    /// Other errors
    Other(String),
}

impl fmt::Display for CrawlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
            Self::BrowserError(msg) => write!(f, "Browser error: {msg}"),
            Self::NetworkError(msg) => write!(f, "Network error: {msg}"),
            Self::Cancelled => write!(f, "Crawl operation was cancelled"),
            Self::Other(msg) => write!(f, "Crawl error: {msg}"),
        }
    }
}

impl std::error::Error for CrawlError {}

impl From<anyhow::Error> for CrawlError {
    fn from(err: anyhow::Error) -> Self {
        // Use {:#} to preserve full error chain with context
        Self::Other(format!("{err:#}"))
    }
}

/// Convenience alias for Result with `CrawlError`
pub type CrawlResult<T> = Result<T, CrawlError>;

/// Represents a crawl progress update
#[derive(Debug, Clone)]
pub enum CrawlProgress {
    /// Initializing the browser
    Initializing,
    /// Browser launched successfully
    BrowserLaunched,
    /// Page navigation started
    NavigationStarted(String),
    /// Page loaded successfully
    PageLoaded(String),
    /// Extracting page data
    ExtractingData,
    /// Saving assets
    SavingAssets,
    /// Taking screenshot
    TakingScreenshot,
    /// Cleanup started
    CleanupStarted,
    /// Crawl completed successfully
    Completed,
    /// Error occurred
    Error(String),
}

use crate::CrawlRequest;
use serde::{Deserialize, Serialize};

/// A trait defining the interface for web crawlers.
pub trait Crawler {
    /// Create a new crawler with the given configuration.
    fn new(config: crate::config::CrawlConfig) -> Self;

    /// Crawl the target URL and store the results.
    /// Returns a `CrawlRequest` that can be awaited for the final result.
    fn crawl(&self) -> CrawlRequest;
}

/// Represents an item in the crawl queue with URL and depth tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlQueue {
    pub url: String,
    pub depth: u8,
}
