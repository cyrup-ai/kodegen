//! Builder methods available for all states
//!
//! This module contains methods that can be called on the builder
//! regardless of its current type state.

use super::builder::CrawlConfigBuilder;

// Methods available for all states after required fields are set
impl<State> CrawlConfigBuilder<State> {
    pub fn target_url(mut self, url: impl Into<String>) -> Self {
        self.target_url = Some(url.into());
        self
    }

    pub fn save_screenshots(mut self, save: bool) -> Self {
        self.save_screenshots = save;
        self
    }

    /// Set browser headless mode (visible vs invisible browser window)
    /// 
    /// By default, the crawler runs in **headless mode** (headless = true) for optimal
    /// performance and compatibility. This is the recommended setting for production use.
    /// 
    /// Set to `false` to enable headed mode, which shows a visible browser window.
    /// This is useful for debugging and development but has significant drawbacks:
    /// 
    /// # Performance Impact
    /// 
    /// Headed mode requires additional resources:
    /// - 20-50% more CPU usage per browser instance
    /// - 100-200 MB more RAM per browser instance
    /// - GUI rendering and window management overhead
    /// - Display buffer allocation
    /// 
    /// # Deployment Limitations
    /// 
    /// Headed mode requires:
    /// - Display server (X11/Wayland on Linux, display on macOS/Windows)
    /// - Not suitable for containers without additional setup
    /// - Not suitable for cloud VMs without GUI
    /// - Requires Xvfb or similar in CI/CD environments
    /// 
    /// # Security Concerns
    /// 
    /// Visible browsers can:
    /// - Display sensitive data on screen
    /// - Be screen-captured unintentionally
    /// - Leak information via window titles
    /// 
    /// # Recommendation
    /// 
    /// **Always use headless mode (default) in production.** Only enable headed mode
    /// for local debugging when you need to visually observe browser behavior.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// // Recommended: Headless mode (default)
    /// let config = CrawlConfig::builder()
    ///     .storage_dir("./output")
    ///     .start_url("https://example.com")
    ///     .build();
    /// 
    /// // For debugging only: Headed mode
    /// let config = CrawlConfig::builder()
    ///     .storage_dir("./output")
    ///     .start_url("https://example.com")
    ///     .headless(false)  // Show visible browser window
    ///     .build();
    /// ```
    pub fn headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    pub fn stealth_mode(mut self, stealth: bool) -> Self {
        self.stealth_mode = stealth;
        self
    }

    pub fn limit(mut self, limit: Option<usize>) -> Self {
        self.limit = limit;
        self
    }

    pub fn only_html(mut self, only_html: bool) -> Self {
        self.only_html = only_html;
        self
    }

    pub fn full_resources(mut self, full_resources: bool) -> Self {
        self.full_resources = full_resources;
        self
    }

    pub fn allow_subdomains(mut self, allow: bool) -> Self {
        self.allow_subdomains = allow;
        self
    }

    pub fn allow_external_domains(mut self, allow: bool) -> Self {
        self.allow_external_domains = allow;
        self
    }

    pub fn save_raw_html(mut self, save: bool) -> Self {
        self.save_raw_html = save;
        self
    }

    pub fn extract_main_content(mut self, extract: bool) -> Self {
        self.extract_main_content = extract;
        self
    }

    pub fn save_markdown(mut self, save: bool) -> Self {
        self.save_markdown = save;
        self
    }

    pub fn content_selector(mut self, selector: Option<String>) -> Self {
        self.content_selector = selector;
        self
    }

    pub fn allowed_domains(mut self, domains: Option<Vec<String>>) -> Self {
        self.allowed_domains = domains;
        self
    }

    pub fn excluded_patterns(mut self, patterns: Option<Vec<String>>) -> Self {
        self.excluded_patterns = patterns;
        self
    }

    pub fn generate_components(mut self, generate: bool) -> Self {
        self.generate_components = generate;
        self
    }

    pub fn progressive(mut self, progressive: bool) -> Self {
        self.progressive = progressive;
        self
    }

    pub fn presentation_style(mut self, style: impl Into<String>) -> Self {
        self.presentation_style = style.into();
        self
    }

    pub fn max_depth(mut self, depth: u8) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn screenshot_quality(mut self, quality: u8) -> Self {
        self.screenshot_quality = quality;
        self
    }

    pub fn search_index_dir(mut self, dir: Option<impl Into<std::path::PathBuf>>) -> Self {
        self.search_index_dir = dir.map(|d| d.into());
        self
    }

    pub fn search_memory_limit(mut self, limit: Option<usize>) -> Self {
        self.search_memory_limit = limit;
        self
    }

    pub fn search_batch_size(mut self, size: Option<usize>) -> Self {
        self.search_batch_size = size;
        self
    }

    /// Set the crawl rate limit in requests per second
    /// 
    /// This controls how fast the crawler will visit pages to be respectful
    /// to target websites. The default is 2.0 RPS.
    /// 
    /// # Arguments
    /// * `rate_rps` - Rate limit in requests per second (e.g., 1.0 for 1 request per second)
    /// 
    /// # Examples
    /// ```
    /// # use citescrape::config::CrawlConfig;
    /// let config = CrawlConfig::builder()
    ///     .storage_dir("./output")
    ///     .start_url("https://example.com")
    ///     .crawl_rate_rps(2.0) // 2 requests per second
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn crawl_rate_rps(mut self, rate_rps: f64) -> Self {
        self.crawl_rate_rps = Some(rate_rps);
        self
    }

    /// Disable crawl rate limiting
    /// 
    /// This allows the crawler to proceed as fast as possible without
    /// any rate limiting. Use with caution as this may overwhelm target websites.
    /// 
    /// # Examples
    /// ```
    /// # use citescrape::config::CrawlConfig;
    /// let config = CrawlConfig::builder()
    ///     .storage_dir("./output")
    ///     .start_url("https://example.com")
    ///     .no_crawl_rate_limit()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn no_crawl_rate_limit(mut self) -> Self {
        self.crawl_rate_rps = None;
        self
    }

    /// Set the maximum image size for inlining as base64
    /// 
    /// Images smaller than this size will be inlined as base64 data URIs.
    /// Images larger than this will be kept as external references.
    /// 
    /// # Arguments
    /// * `max_bytes` - Maximum size in bytes (e.g., 100_000 for 100KB)
    /// 
    /// # Examples
    /// ```
    /// # use citescrape::config::CrawlConfig;
    /// let config = CrawlConfig::builder()
    ///     .storage_dir("./output")
    ///     .start_url("https://example.com")
    ///     .max_inline_image_size_bytes(100_000) // 100KB limit
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn max_inline_image_size_bytes(mut self, max_bytes: usize) -> Self {
        self.max_inline_image_size_bytes = Some(max_bytes);
        self
    }

    /// Inline all images regardless of size
    /// 
    /// This is the default behavior - all images will be converted to base64
    /// and inlined in the HTML, regardless of their size.
    pub fn inline_all_images(mut self) -> Self {
        self.max_inline_image_size_bytes = None;
        self
    }

    /// Set the maximum size for the deferred queue
    /// 
    /// Controls how many rate-limited URLs can be queued for retry.
    /// Set to None for unlimited (not recommended), or Some(size) to limit.
    /// 
    /// Default: Some(10_000)
    pub fn max_deferred_queue_size(mut self, size: Option<usize>) -> Self {
        self.max_deferred_queue_size = size;
        self
    }

    /// Enable or disable etag-based cache validation
    ///
    /// When enabled, the crawler checks if cached files exist and validates
    /// them using HTTP ETags. If the etag matches, content extraction is skipped.
    ///
    /// Default: true
    pub fn enable_cache_validation(mut self, enable: bool) -> Self {
        self.enable_cache_validation = enable;
        self
    }
    
    /// Force re-crawl ignoring any cached files
    ///
    /// When true, all pages are re-crawled and re-extracted even if valid
    /// cached versions exist.
    ///
    /// Default: false
    pub fn ignore_cache(mut self, ignore: bool) -> Self {
        self.ignore_cache = ignore;
        self
    }

    /// Set timeout for cache validation etag checks
    ///
    /// When validating cached content via HTTP ETags, this timeout determines
    /// how long to wait for the network response event containing the etag.
    ///
    /// Increase this value for slow servers or networks to prevent false cache
    /// misses. Decrease to fail faster, but risk missing valid cached content.
    ///
    /// Default: 15 seconds
    ///
    /// # Arguments
    /// * `timeout_secs` - Timeout in seconds (e.g., 30 for 30 seconds)
    ///
    /// # Examples
    /// ```
    /// # use citescrape::config::CrawlConfig;
    /// let config = CrawlConfig::builder()
    ///     .storage_dir("./output")
    ///     .start_url("https://example.com")
    ///     .cache_validation_timeout_secs(30) // Wait up to 30 seconds
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn cache_validation_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.cache_validation_timeout_secs = Some(timeout_secs);
        self
    }

    /// Enable or disable circuit breaker pattern for domain-level failure detection
    ///
    /// When enabled, the crawler tracks failures per domain and will temporarily
    /// stop attempting requests to domains that exceed the failure threshold.
    /// This helps prevent wasting resources on consistently failing domains.
    ///
    /// Default: true
    ///
    /// # Examples
    /// ```
    /// # use citescrape::config::CrawlConfig;
    /// let config = CrawlConfig::builder()
    ///     .storage_dir("./output")
    ///     .start_url("https://example.com")
    ///     .circuit_breaker_enabled(true)
    ///     .circuit_breaker_failure_threshold(3)
    ///     .circuit_breaker_retry_delay_secs(60)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn circuit_breaker_enabled(mut self, enabled: bool) -> Self {
        self.circuit_breaker_enabled = enabled;
        self
    }

    /// Set the failure threshold for circuit breaker
    ///
    /// Number of consecutive failures before a domain's circuit opens.
    /// Once the threshold is reached, the circuit breaker will stop
    /// attempting requests to that domain until the retry delay expires.
    ///
    /// Default: 5
    ///
    /// # Arguments
    /// * `threshold` - Number of failures before circuit opens (e.g., 5)
    pub fn circuit_breaker_failure_threshold(mut self, threshold: u32) -> Self {
        self.circuit_breaker_failure_threshold = threshold;
        self
    }

    /// Set the retry delay for circuit breaker
    ///
    /// How long to wait (in seconds) before retrying a domain after its
    /// circuit opens. During this time, requests to the domain will be skipped.
    /// After the delay expires, the circuit enters half-open state and allows
    /// a limited number of test requests.
    ///
    /// Default: 300 seconds (5 minutes)
    ///
    /// # Arguments
    /// * `delay_secs` - Delay in seconds before retry (e.g., 120 for 2 minutes)
    pub fn circuit_breaker_retry_delay_secs(mut self, delay_secs: u64) -> Self {
        self.circuit_breaker_retry_delay_secs = delay_secs;
        self
    }
}