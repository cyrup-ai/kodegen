//! Core configuration types for web crawling
//!
//! This module contains the main `CrawlConfig` struct and its associated types
//! that define the configuration parameters for web crawling operations.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration struct for web crawling operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlConfig {
    pub(crate) storage_dir: PathBuf,
    pub(crate) only_html: bool,
    pub(crate) full_resources: bool,
    pub(crate) start_url: String,
    pub(crate) target_url: String,
    pub(crate) limit: Option<usize>,
    pub(crate) screenshot_quality: u8,
    pub(crate) stealth_mode: bool,
    pub(crate) allow_subdomains: bool,
    pub(crate) allow_external_domains: bool,
    pub(crate) save_screenshots: bool,
    pub(crate) save_json: bool,
    pub(crate) save_raw_html: bool,
    pub(crate) extract_main_content: bool,
    pub(crate) save_markdown: bool,
    pub(crate) headless: bool,
    pub(crate) content_selector: Option<String>,
    pub(crate) allowed_domains: Option<Vec<String>>,
    pub(crate) excluded_patterns: Option<Vec<String>>,

    /// Compiled regex patterns from `excluded_patterns`
    /// Pre-compiled at config creation to avoid hot-path regex compilation
    #[serde(skip)]
    pub(crate) excluded_patterns_compiled: Vec<regex::Regex>,

    pub(crate) generate_components: bool,
    pub(crate) progressive: bool,
    pub(crate) presentation_style: String,
    pub(crate) max_depth: u8,
    pub(crate) search_index_dir: Option<PathBuf>,
    pub(crate) search_memory_limit: Option<usize>,
    pub(crate) search_batch_size: Option<usize>,
    pub(crate) crawl_rate_rps: Option<f64>,
    /// Maximum size in bytes for inlining images as base64.
    /// Images larger than this will be kept as external references.
    /// Default is None (all images are inlined).
    pub(crate) max_inline_image_size_bytes: Option<usize>,
    pub(crate) max_deferred_queue_size: Option<usize>,

    /// Enable etag-based cache validation for incremental crawls
    pub(crate) enable_cache_validation: bool,

    /// Force re-crawl even if cached files exist (ignore cache)
    pub(crate) ignore_cache: bool,

    /// Timeout in seconds for cache validation etag checks
    ///
    /// When validating cached content via HTTP `ETags`, this timeout
    /// determines how long to wait for the network response event.
    ///
    /// Longer timeouts are needed for slow servers or networks.
    /// Shorter timeouts fail faster but may cause false cache misses.
    ///
    /// Default: 15 seconds
    pub(crate) cache_validation_timeout_secs: Option<u64>,

    /// Timeout in seconds for `page.goto()` operations
    ///
    /// Controls how long to wait for page navigation to complete.
    /// Prevents hangs on slow DNS, unresponsive servers, or streaming content.
    ///
    /// Default: 30 seconds
    pub(crate) page_load_timeout_secs: Option<u64>,

    /// Timeout in seconds for `page.wait_for_navigation()` operations
    ///
    /// Controls how long to wait for page load events.
    /// Prevents hangs on pages with long-polling, streaming, or infinite JS loops.
    ///
    /// Default: 30 seconds
    pub(crate) navigation_timeout_secs: Option<u64>,

    /// Timeout in seconds for `page.event_listener()` setup
    ///
    /// Controls how long to wait for event listener initialization.
    /// Prevents hangs when setting up network event listeners.
    ///
    /// Default: 10 seconds
    pub(crate) event_timeout_secs: Option<u64>,

    /// Enable circuit breaker for domain-level failure detection
    ///
    /// When enabled, the crawler will track failures per domain and
    /// stop attempting requests to consistently failing domains.
    ///
    /// Default: true
    pub(crate) circuit_breaker_enabled: bool,

    /// Number of consecutive failures before opening circuit
    ///
    /// After this many consecutive failures to a domain, the circuit
    /// breaker will transition to Open state and skip further requests
    /// to that domain (until retry timeout expires).
    ///
    /// Default: 5
    pub(crate) circuit_breaker_failure_threshold: u32,

    /// Delay in seconds before retrying a failed domain
    ///
    /// When a circuit is Open due to failures, this is how long to wait
    /// before attempting the domain again in `HalfOpen` state.
    ///
    /// Default: 300 seconds (5 minutes)
    pub(crate) circuit_breaker_retry_delay_secs: u64,

    /// Optional event bus for publishing crawl events
    ///
    /// When set, the crawler will publish `CrawlEvent` updates to this bus.
    /// Subscribers can use `subscribe()` or `subscribe_filtered()` to receive events.
    #[serde(skip)]
    pub(crate) event_bus: Option<std::sync::Arc<crate::crawl_events::CrawlEventBus>>,

    /// Optional indexing service for real-time search index updates
    ///
    /// When set, the crawler will send document updates to this indexing service
    /// for incremental search index updates during crawling.
    #[serde(skip)]
    pub(crate) indexing_sender: Option<std::sync::Arc<crate::search::IndexingSender>>,

    /// Maximum number of pages to crawl concurrently
    /// Default: 10, Range: 1-100
    pub(crate) max_concurrent_pages: Option<usize>,

    /// Maximum concurrent pages per domain (prevents rate limiting)
    /// Default: 2, Range: 1-10
    pub(crate) max_concurrent_per_domain: Option<usize>,

    /// Chrome user data directory path for browser profile isolation
    /// When set, ensures each crawl session uses a unique Chrome profile.
    /// This prevents profile lock contention in long-running server processes.
    #[serde(skip)]
    pub(crate) chrome_data_dir: Option<PathBuf>,

    /// Enable gzip compression for saved files (markdown, html, json, screenshots)
    /// When true, files are saved with .gz extension and compressed
    /// When false (default), files are saved uncompressed for easier inspection
    /// Default: false
    pub(crate) compress_output: bool,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from("./output"),
            start_url: String::new(),
            target_url: String::new(),
            only_html: false,
            headless: true,
            progressive: false,
            presentation_style: String::new(),
            max_depth: 3,
            content_selector: None,
            allowed_domains: None,
            excluded_patterns: None,
            excluded_patterns_compiled: Vec::new(),
            generate_components: false,
            full_resources: true,
            limit: None,
            screenshot_quality: 80,
            stealth_mode: false,
            allow_subdomains: false,
            allow_external_domains: false,
            save_screenshots: true,
            save_json: true,
            save_raw_html: false,
            extract_main_content: true,
            save_markdown: true,
            search_index_dir: None,
            search_memory_limit: None, // Set dynamically based on available memory, up to 4GB
            search_batch_size: Some(1000),
            crawl_rate_rps: Some(2.0), // Default to 2 requests per second for respectful crawling
            max_inline_image_size_bytes: None, // Default to None (inline all images)
            max_deferred_queue_size: Some(10_000),
            enable_cache_validation: true,
            ignore_cache: false,
            cache_validation_timeout_secs: Some(15),
            page_load_timeout_secs: Some(30),
            navigation_timeout_secs: Some(30),
            event_timeout_secs: Some(10),
            circuit_breaker_enabled: true,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_retry_delay_secs: 300,
            event_bus: None,
            indexing_sender: None,
            max_concurrent_pages: Some(10),
            max_concurrent_per_domain: Some(2),
            chrome_data_dir: None,
            compress_output: false, // Default to uncompressed for easier inspection
        }
    }
}

// Constructor
impl CrawlConfig {
    /// Attach an event bus for real-time crawl events
    #[must_use]
    pub fn with_event_bus(
        mut self,
        bus: std::sync::Arc<crate::crawl_events::CrawlEventBus>,
    ) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Get the event bus if attached
    #[must_use]
    pub fn event_bus(&self) -> Option<&std::sync::Arc<crate::crawl_events::CrawlEventBus>> {
        self.event_bus.as_ref()
    }

    /// Attach an indexing sender for real-time search index updates
    ///
    /// When attached, the crawler will automatically send document updates
    /// to the incremental indexing service during crawling.
    ///
    /// # Example
    /// ```rust
    /// let config = CrawlConfig::builder()
    ///     .start_url("https://example.com")
    ///     .with_indexing_sender(indexing_sender)
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_indexing_sender(
        mut self,
        sender: std::sync::Arc<crate::search::IndexingSender>,
    ) -> Self {
        self.indexing_sender = Some(sender);
        self
    }

    /// Get the indexing sender if configured
    #[must_use]
    pub fn indexing_sender(&self) -> Option<&std::sync::Arc<crate::search::IndexingSender>> {
        self.indexing_sender.as_ref()
    }

    /// Set Chrome user data directory for browser profile isolation
    ///
    /// When set, the browser will use this specific directory for its user data,
    /// ensuring profile isolation between crawl sessions. This prevents Chrome profile
    /// lock contention in long-running server processes where multiple crawls may run
    /// concurrently or sequentially.
    ///
    /// # Example
    /// ```rust
    /// let chrome_dir = std::env::temp_dir().join(format!("chrome_{}", session_id));
    /// let config = CrawlConfig::builder()
    ///     .start_url("https://example.com")
    ///     .with_chrome_data_dir(chrome_dir)
    ///     .build();
    /// ```
    #[must_use]
    pub fn with_chrome_data_dir(mut self, dir: PathBuf) -> Self {
        self.chrome_data_dir = Some(dir);
        self
    }

    /// Get the Chrome user data directory if configured
    #[must_use]
    pub fn chrome_data_dir(&self) -> Option<&PathBuf> {
        self.chrome_data_dir.as_ref()
    }

    /// Get the pre-compiled excluded patterns
    ///
    /// These patterns are compiled once at config creation time
    /// to avoid repeated regex compilation in the hot path.
    #[must_use]
    pub fn excluded_patterns_compiled(&self) -> &[regex::Regex] {
        &self.excluded_patterns_compiled
    }
}
