//! Getter methods for CrawlConfig
//!
//! This module provides all the accessor methods for retrieving configuration
//! values from a CrawlConfig instance.

use std::path::PathBuf;

use super::types::CrawlConfig;

impl CrawlConfig {
    pub fn storage_dir(&self) -> &PathBuf {
        &self.storage_dir
    }

    pub fn start_url(&self) -> &str {
        &self.start_url
    }

    pub fn target_url(&self) -> &str {
        &self.target_url
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    pub fn screenshot_quality(&self) -> u8 {
        self.screenshot_quality
    }

    pub fn stealth_mode(&self) -> bool {
        self.stealth_mode
    }

    pub fn allow_subdomains(&self) -> bool {
        self.allow_subdomains
    }

    pub fn allow_external_domains(&self) -> bool {
        self.allow_external_domains
    }

    pub fn save_screenshots(&self) -> bool {
        self.save_screenshots
    }

    pub fn save_raw_html(&self) -> bool {
        self.save_raw_html
    }

    pub fn extract_main_content(&self) -> bool {
        self.extract_main_content
    }

    pub fn save_markdown(&self) -> bool {
        self.save_markdown
    }

    pub fn save_json(&self) -> bool {
        self.save_json
    }

    pub fn headless(&self) -> bool {
        self.headless
    }

    pub fn content_selector(&self) -> Option<&str> {
        self.content_selector.as_deref()
    }

    pub fn allowed_domains(&self) -> Option<&Vec<String>> {
        self.allowed_domains.as_ref()
    }

    pub fn excluded_patterns(&self) -> Option<&Vec<String>> {
        self.excluded_patterns.as_ref()
    }

    pub fn generate_components(&self) -> bool {
        self.generate_components
    }

    pub fn progressive(&self) -> bool {
        self.progressive
    }

    pub fn presentation_style(&self) -> &str {
        &self.presentation_style
    }

    pub fn max_depth(&self) -> u8 {
        self.max_depth
    }

    pub fn search_index_dir(&self) -> PathBuf {
        self.search_index_dir.clone().unwrap_or_else(|| {
            self.storage_dir.join("search_index")
        })
    }

    pub fn search_memory_limit(&self) -> usize {
        self.search_memory_limit.unwrap_or_else(|| {
            // Calculate dynamic memory limit based on available system memory
            // Up to 4GB max, but adapt to available memory
            let available_memory = get_available_memory();
            let max_limit = 4_294_967_296; // 4GB
            let conservative_limit = available_memory / 4; // Use 25% of available memory
            std::cmp::min(max_limit, conservative_limit)
        })
    }

    pub fn search_batch_size(&self) -> usize {
        self.search_batch_size.unwrap_or(1000)
    }

    /// Get the crawl rate limit in requests per second
    /// 
    /// Returns the configured rate limit, or None if rate limiting is disabled.
    /// The default rate limit is 2.0 RPS for respectful crawling.
    pub fn crawl_rate_rps(&self) -> Option<f64> {
        self.crawl_rate_rps
    }

    /// Get the maximum image size for inlining as base64
    /// 
    /// Returns None if all images should be inlined regardless of size,
    /// or Some(bytes) to limit inlining to images smaller than this size.
    pub fn max_inline_image_size_bytes(&self) -> Option<usize> {
        self.max_inline_image_size_bytes
    }

    /// Get the maximum size of the deferred queue for rate-limited URLs
    /// 
    /// Returns the configured maximum number of URLs that can be held in the
    /// deferred queue waiting for retry. When this limit is reached, additional
    /// rate-limited URLs will be dropped with a warning.
    /// 
    /// Default is 10,000 URLs, which provides reasonable memory bounds while
    /// allowing large crawls to handle temporary rate limiting effectively.
    pub fn max_deferred_queue_size(&self) -> usize {
        self.max_deferred_queue_size.unwrap_or(10_000)
    }

    /// Check if cache validation is enabled
    pub fn enable_cache_validation(&self) -> bool {
        self.enable_cache_validation
    }
    
    /// Check if cache should be ignored (force re-crawl)
    pub fn ignore_cache(&self) -> bool {
        self.ignore_cache
    }

    /// Get the cache validation timeout in seconds
    ///
    /// Returns the configured timeout for etag-based cache validation checks.
    /// If None, defaults to 15 seconds.
    pub fn cache_validation_timeout_secs(&self) -> u64 {
        self.cache_validation_timeout_secs.unwrap_or(15)
    }

    /// Get the page load timeout in seconds
    ///
    /// Returns the configured timeout for page.goto() operations.
    /// If None, defaults to 30 seconds.
    pub fn page_load_timeout_secs(&self) -> u64 {
        self.page_load_timeout_secs.unwrap_or(30)
    }

    /// Get the navigation timeout in seconds
    ///
    /// Returns the configured timeout for page.wait_for_navigation() operations.
    /// If None, defaults to 30 seconds.
    pub fn navigation_timeout_secs(&self) -> u64 {
        self.navigation_timeout_secs.unwrap_or(30)
    }

    /// Get the event listener timeout in seconds
    ///
    /// Returns the configured timeout for page.event_listener() setup.
    /// If None, defaults to 10 seconds.
    pub fn event_timeout_secs(&self) -> u64 {
        self.event_timeout_secs.unwrap_or(10)
    }

    /// Check if circuit breaker is enabled
    ///
    /// Returns true if the circuit breaker should track domain failures
    /// and short-circuit consistently failing domains.
    pub fn circuit_breaker_enabled(&self) -> bool {
        self.circuit_breaker_enabled
    }

    /// Get the circuit breaker failure threshold
    ///
    /// Returns the number of consecutive failures before opening the circuit.
    /// Default is 5.
    pub fn circuit_breaker_failure_threshold(&self) -> u32 {
        self.circuit_breaker_failure_threshold
    }

    /// Get the circuit breaker retry delay in seconds
    ///
    /// Returns how long to wait before retrying a failed domain.
    /// Default is 300 seconds (5 minutes).
    pub fn circuit_breaker_retry_delay_secs(&self) -> u64 {
        self.circuit_breaker_retry_delay_secs
    }

    /// Get the maximum number of pages to crawl concurrently
    ///
    /// Returns the configured concurrency limit.
    /// Default is 10, range is 1-100.
    pub fn max_concurrent_pages(&self) -> usize {
        self.max_concurrent_pages.unwrap_or(10)
    }

    /// Get the maximum concurrent pages per domain
    ///
    /// Returns the configured per-domain concurrency limit to prevent rate limiting.
    /// Default is 2, range is 1-10.
    pub fn max_concurrent_per_domain(&self) -> usize {
        self.max_concurrent_per_domain.unwrap_or(2)
    }
}

fn get_available_memory() -> usize {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<usize>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("sysctl")
            .args(["hw.memsize"])
            .output()
            && let Ok(output_str) = String::from_utf8(output.stdout)
            && let Some(mem_str) = output_str.split_whitespace().nth(1)
            && let Ok(total_memory) = mem_str.parse::<usize>()
        {
            // Estimate available as 75% of total (conservative)
            return (total_memory * 3) / 4;
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("wmic")
            .args(&["OS", "get", "TotalVisibleMemorySize", "/value"])
            .output()
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                for line in output_str.lines() {
                    if line.starts_with("TotalVisibleMemorySize=") {
                        if let Some(kb_str) = line.split('=').nth(1) {
                            if let Ok(kb) = kb_str.parse::<usize>() {
                                // Estimate available as 75% of total (conservative)
                                return (kb * 1024 * 3) / 4;
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: 2GB if we can't determine system memory
    2_147_483_648
}