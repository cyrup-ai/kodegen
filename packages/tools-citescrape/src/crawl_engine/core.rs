//! Core crawling logic with integrated event bus support
//!
//! This module contains the canonical crawling implementation with
//! trait-based abstraction for progress reporting AND event bus integration.

use anyhow::{Context, Result};
use bloomfilter::Bloom;
use chromiumoxide::browser::Browser;
use dashmap::DashSet;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use log::{debug, error, info, warn};
use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

use super::crawl_types::CrawlQueue;
use super::{CircuitBreaker, DomainLimiter, extract_domain};
use crate::browser_setup::launch_browser;
use crate::config::CrawlConfig;
use crate::crawl_events::{
    CrawlEventBus,
    types::{CrawlEvent, PageCrawlMetadata},
};
use crate::page_extractor;
use crate::page_extractor::link_rewriter::LinkRewriter;

use super::link_processor::{CrawlState, process_page_links};
use crate::content_saver;
use crate::content_saver::markdown_converter::{ConversionOptions, convert_html_to_markdown};
use html2md;

/// Helper function to wrap async page operations with explicit timeout
///
/// Prevents indefinite hangs on page operations by applying `tokio::time::timeout`.
/// Returns proper error messages distinguishing between timeout and operation failures.
///
/// # Arguments
/// * `operation` - The async Future to execute with a timeout
/// * `timeout_secs` - Timeout duration in seconds
/// * `operation_name` - Human-readable name for error messages
///
/// # Returns
/// * `Ok(T)` - Operation completed successfully
/// * `Err` - Either the operation failed or the timeout was reached
async fn with_page_timeout<F, T>(operation: F, timeout_secs: u64, operation_name: &str) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match tokio::time::timeout(Duration::from_secs(timeout_secs), operation).await {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!(
            "{operation_name} timeout after {timeout_secs} seconds"
        )),
    }
}

/// Trait for reporting crawl progress at key lifecycle events
///
/// Implementations can send updates to channels, log to console, update UI, etc.
/// This abstraction allows the same core crawl logic to support both simple
/// and progress-reporting APIs.
pub trait ProgressReporter: Send + Sync {
    /// Report that browser initialization has started
    fn report_initializing(&self);

    /// Report that the browser has launched successfully
    fn report_browser_launched(&self);

    /// Report that navigation to a URL has started
    fn report_navigation_started(&self, url: &str);

    /// Report that a page has loaded successfully
    fn report_page_loaded(&self, url: &str);

    /// Report that page data extraction has started
    fn report_extracting_data(&self);

    /// Report that a screenshot is being captured
    fn report_taking_screenshot(&self);

    /// Report that cleanup has started
    fn report_cleanup_started(&self);

    /// Report that the crawl has completed successfully
    fn report_completed(&self);

    /// Report an error that occurred during crawling
    fn report_error(&self, error: &str);
}

/// Progress reporter that does nothing
///
/// Used by the simple `crawl_impl()` API that doesn't need progress updates.
/// All methods are no-ops and will be inlined away by the compiler.
#[derive(Debug, Clone, Copy)]
pub struct NoOpProgress;

impl ProgressReporter for NoOpProgress {
    #[inline(always)]
    fn report_initializing(&self) {}

    #[inline(always)]
    fn report_browser_launched(&self) {}

    #[inline(always)]
    fn report_navigation_started(&self, _url: &str) {}

    #[inline(always)]
    fn report_page_loaded(&self, _url: &str) {}

    #[inline(always)]
    fn report_extracting_data(&self) {}

    #[inline(always)]
    fn report_taking_screenshot(&self) {}

    #[inline(always)]
    fn report_cleanup_started(&self) {}

    #[inline(always)]
    fn report_completed(&self) {}

    #[inline(always)]
    fn report_error(&self, _error: &str) {}
}

/// Main crawl orchestration with event bus integration
///
/// This function implements the complete multi-page crawling logic:
/// - Browser initialization
/// - Breadth-first queue-based crawling with depth control
/// - Page processing and content extraction
/// - Link discovery and filtering
/// - Progress reporting via `ProgressReporter` trait
/// - Event publishing via optional `CrawlEventBus`
///
/// # Arguments
/// * `config` - Crawl configuration
/// * `link_rewriter` - Link rewriting manager
/// * `chrome_data_dir` - Optional Chrome data directory
/// * `progress` - Progress reporter (`NoOpProgress`)
/// * `event_bus` - Optional event bus for crawl events
pub async fn crawl_pages<P: ProgressReporter>(
    config: CrawlConfig,
    link_rewriter: LinkRewriter,
    chrome_data_dir: Option<PathBuf>,
    progress: P,
    event_bus: Option<Arc<CrawlEventBus>>,
) -> Result<Option<PathBuf>> {
    // Extract indexing_sender early to avoid borrow issues
    let indexing_sender = config.indexing_sender().map(Arc::clone);

    let start_time = Instant::now();

    progress.report_initializing();

    // Initialize thread-safe crawl queue
    let queue = Arc::new(tokio::sync::Mutex::new({
        let mut q = VecDeque::new();
        q.push_back(CrawlQueue {
            url: config.start_url.clone(),
            depth: 0,
        });
        q
    }));

    // Lock-free visited set (replaces Bloom filter for thread-safety)
    // DashSet provides concurrent access without locks, ideal for multi-task crawling
    let visited: Arc<DashSet<String>> = Arc::new(DashSet::new());

    // Thread-safe circuit breaker for domain-level failure detection
    let circuit_breaker = if config.circuit_breaker_enabled() {
        Some(Arc::new(tokio::sync::Mutex::new(CircuitBreaker::new(
            config.circuit_breaker_failure_threshold(),
            2, // success_threshold: close circuit after 2 consecutive successes
            Duration::from_secs(config.circuit_breaker_retry_delay_secs()),
        ))))
    } else {
        None
    };

    // Atomic counter for total pages crawled
    let total_pages = Arc::new(AtomicUsize::new(0));

    // Publish CrawlStarted event (CRITICAL - fail if this doesn't work)
    if let Some(bus) = &event_bus {
        let event = CrawlEvent::crawl_started(
            config.start_url.clone(),
            config.storage_dir.clone(),
            u32::from(config.max_depth),
        );
        bus.publish(event)
            .await
            .context("Failed to publish CrawlStarted event - event bus may be shutdown or full")?;
    }

    // Setup browser - find system Chrome or download managed Chromium
    // launch_browser handles:
    // - Finding Chrome/Chromium executable (system paths + 'which' command)
    // - Downloading managed Chromium if not found (with proper dir creation)
    // - Configuring browser with stealth mode arguments
    // - Spawning handler task to drive CDP connection
    // - Using unique chrome_data_dir per session (if configured) to prevent profile lock contention

    // DEBUG: Log chrome_data_dir from config
    eprintln!(
        "DEBUG core.rs: config.chrome_data_dir() = {:?}",
        config.chrome_data_dir()
    );
    let chrome_dir_param = config.chrome_data_dir().cloned();
    eprintln!("DEBUG core.rs: Passing to launch_browser: {chrome_dir_param:?}");

    let (browser, handler_task) = launch_browser(config.headless(), chrome_dir_param)
        .await
        .context("Failed to launch browser")?;

    progress.report_browser_launched();

    // Wrap browser in Arc for sharing across concurrent tasks
    let browser = Arc::new(browser);

    // Concurrency control
    let concurrency = config.max_concurrent_pages();
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let domain_limiter = Arc::new(DomainLimiter::new(config.max_concurrent_per_domain()));

    // Main concurrent crawl loop
    let mut active_tasks = FuturesUnordered::new();

    loop {
        // Fill up to concurrency limit
        while active_tasks.len() < concurrency {
            // Pop next item from queue
            let item = {
                let mut q = queue.lock().await;
                match q.pop_front() {
                    Some(item) => item,
                    None if active_tasks.is_empty() => break, // All done
                    None => break,                            // Wait for tasks to complete
                }
            };

            // Check limit
            if let Some(limit) = config.limit
                && total_pages.load(Ordering::Relaxed) >= limit
            {
                info!("Reached page limit of {limit}");
                break;
            }

            // Check if already visited (lock-free check)
            if !visited.insert(item.url.clone()) {
                continue; // Already visited
            }

            // Acquire global semaphore permit (limits total concurrency)
            let permit = if let Ok(p) = semaphore.clone().acquire_owned().await {
                p
            } else {
                error!("Semaphore closed unexpectedly");
                continue;
            };

            // Acquire domain-specific permit (prevents rate limiting)
            let domain = match extract_domain(&item.url) {
                Ok(d) => d,
                Err(e) => {
                    warn!("Failed to extract domain from {}: {}", item.url, e);
                    continue;
                }
            };
            let domain_permit = domain_limiter.acquire(domain).await;

            // Clone all shared state for the task
            let browser = Arc::clone(&browser);
            let config = config.clone();
            let link_rewriter = link_rewriter.clone();
            let event_bus = event_bus.clone();
            let circuit_breaker = circuit_breaker.clone();
            let total_pages = Arc::clone(&total_pages);
            let queue = Arc::clone(&queue);
            let _visited = Arc::clone(&visited);
            let indexing_sender = indexing_sender.clone();

            // Spawn concurrent task
            let task = tokio::spawn(async move {
                let _permit = permit; // Hold until task completes
                let _domain_permit = domain_permit; // Hold until task completes

                // Process single page with full error handling
                match process_single_page(
                    browser,
                    item,
                    config,
                    link_rewriter,
                    event_bus,
                    circuit_breaker,
                    total_pages,
                    queue,
                    indexing_sender,
                )
                .await
                {
                    Ok(url) => {
                        debug!("Successfully crawled: {url}");
                        Ok(url)
                    }
                    Err(e) => {
                        debug!("Failed to crawl page: {e}");
                        Err(e)
                    }
                }
            });

            active_tasks.push(task);
        }

        // Wait for at least one task to complete
        match active_tasks.next().await {
            Some(Ok(result)) => match result {
                Ok(url) => {
                    debug!("Completed crawling: {url}");
                }
                Err(e) => {
                    warn!("Crawl task failed: {e}");
                }
            },
            Some(Err(e)) => {
                error!("Task panicked: {e}");
            }
            None => break, // All tasks completed
        }

        // Check if done
        let remaining = queue.lock().await.len();
        if remaining == 0 && active_tasks.is_empty() {
            break;
        }
    }

    // Publish LinkRewriteCompleted if rewriting happened
    let urls_registered = link_rewriter.get_registration_count().await;
    let total_pages_final = total_pages.load(Ordering::Relaxed);
    if urls_registered > 0 {
        if let Some(bus) = &event_bus {
            let event = CrawlEvent::link_rewrite_completed(
                config.start_url.clone(),
                total_pages_final,
                urls_registered,
            );
            if let Err(e) = bus.publish(event).await {
                warn!("Failed to publish LinkRewriteCompleted event: {e}");
            }
        }
        info!("Link rewriting enabled: {urls_registered} URLs registered for local navigation");
    }

    // Publish CrawlCompleted event (CRITICAL - this is the final summary)
    if let Some(bus) = &event_bus {
        let event =
            CrawlEvent::crawl_completed(total_pages_final, urls_registered, start_time.elapsed());
        bus.publish(event).await.context(
            "Failed to publish CrawlCompleted event - event bus may be shutdown or full",
        )?;

        // Report final metrics before shutdown
        let metrics = bus.metrics().snapshot();
        if metrics.events_failed > 0 || metrics.events_dropped > 0 {
            warn!(
                "Event bus metrics - Published: {}, Dropped: {}, Failed: {}, Success rate: {:.1}%",
                metrics.events_published,
                metrics.events_dropped,
                metrics.events_failed,
                metrics.success_rate() * 100.0
            );
        } else {
            debug!(
                "Event bus metrics - Published: {} events to {} subscribers",
                metrics.events_published, metrics.active_subscribers
            );
        }

        // Graceful shutdown
        bus.shutdown_gracefully(crate::crawl_events::types::ShutdownReason::CrawlCompleted)
            .await;
    }

    progress.report_cleanup_started();

    // Clean up browser and data
    let chrome_data_dir_path = chrome_data_dir
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Chrome data directory path not available for cleanup"))?;

    // Abort browser handler task to prevent resource leak
    info!("Aborting browser handler task");
    handler_task.abort();
    if let Err(e) = handler_task.await
        && !e.is_cancelled()
    {
        warn!("Handler task failed during abort: {e}");
    }

    // Try to unwrap browser from Arc - if there are still references, skip cleanup
    let browser_for_cleanup = match Arc::try_unwrap(browser) {
        Ok(b) => Some(b),
        Err(arc) => {
            warn!(
                "Browser still has {} strong references, cleanup will happen on drop",
                Arc::strong_count(&arc)
            );
            None
        }
    };

    if let Some(browser_owned) = browser_for_cleanup {
        match super::cleanup::cleanup_browser_and_data(browser_owned, chrome_data_dir_path).await {
            Ok(super::cleanup::CleanupResult::Success) => {
                debug!("Browser and data cleanup completed successfully");
            }
            Ok(super::cleanup::CleanupResult::PartialFailure(errors)) => {
                warn!("Cleanup completed with failures: {errors:?}");
                for error in &errors {
                    progress.report_error(&format!("Cleanup error: {error}"));
                }
            }
            Err(e) => {
                warn!("Cleanup failed: {e}");
                progress.report_error(&format!("Cleanup failed: {e}"));
            }
        }
    }

    progress.report_completed();

    Ok(chrome_data_dir)
}

/// Process a single page concurrently
#[allow(clippy::too_many_arguments)]
async fn process_single_page(
    browser: Arc<Browser>,
    item: CrawlQueue,
    config: CrawlConfig,
    link_rewriter: LinkRewriter,
    event_bus: Option<Arc<CrawlEventBus>>,
    circuit_breaker: Option<Arc<tokio::sync::Mutex<CircuitBreaker>>>,
    total_pages: Arc<AtomicUsize>,
    queue: Arc<tokio::sync::Mutex<VecDeque<CrawlQueue>>>,
    indexing_sender: Option<Arc<crate::search::IndexingSender>>,
) -> Result<String> {
    let page_start = Instant::now();

    // Apply rate limiting
    if let Some(rate) = config.crawl_rate_rps {
        use super::rate_limiter::{RateLimitDecision, check_crawl_rate_limit};
        match check_crawl_rate_limit(&item.url, rate).await {
            RateLimitDecision::Deny { retry_after } => {
                debug!("Rate limited, sleeping for {:?}: {}", retry_after, item.url);
                tokio::time::sleep(retry_after).await;
            }
            RateLimitDecision::Allow => {}
        }
    }

    // Check circuit breaker
    if let Some(ref cb) = circuit_breaker {
        let domain = extract_domain(&item.url).map_err(|e| anyhow::anyhow!("{e}"))?;
        let mut cb_guard = cb.lock().await;
        if !cb_guard.should_attempt(&domain) {
            debug!("Circuit breaker OPEN, skipping: {}", item.url);
            return Err(anyhow::anyhow!("Circuit breaker open for domain"));
        }
    }

    info!("Crawling [depth {}]: {}", item.depth, item.url);

    // Create page
    let page = match browser.new_page("about:blank").await {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to create page for {}: {}", item.url, e);
            if let Some(ref cb) = circuit_breaker
                && let Ok(domain) = extract_domain(&item.url)
            {
                let mut cb_guard = cb.lock().await;
                cb_guard.record_failure(&domain, &e.to_string());
            }
            return Err(e.into());
        }
    };

    // Apply page enhancements
    match super::page_enhancer::enhance_page(page.clone()).await {
        Ok(()) => debug!("Page enhancements applied for: {}", item.url),
        Err(e) => warn!("Failed to apply page enhancements for {}: {}", item.url, e),
    }

    // Navigate to page
    let page_load_timeout = config.page_load_timeout_secs();
    if let Err(e) = with_page_timeout(
        async {
            page.goto(&item.url)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        },
        page_load_timeout,
        "Page navigation",
    )
    .await
    {
        warn!("Navigation failed for {}: {}", item.url, e);
        if let Some(ref cb) = circuit_breaker
            && let Ok(domain) = extract_domain(&item.url)
        {
            let mut cb_guard = cb.lock().await;
            cb_guard.record_failure(&domain, &e.to_string());
        }
        return Err(e);
    }

    // Wait for page load
    let navigation_timeout = config.navigation_timeout_secs();
    if let Err(e) = with_page_timeout(
        async {
            page.wait_for_navigation()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))
        },
        navigation_timeout,
        "Page load",
    )
    .await
    {
        warn!("Page load failed for {}: {}", item.url, e);
        if let Some(ref cb) = circuit_breaker
            && let Ok(domain) = extract_domain(&item.url)
        {
            let mut cb_guard = cb.lock().await;
            cb_guard.record_failure(&domain, &e.to_string());
        }
        return Err(e);
    }

    // Extract page data
    let extract_config = crate::page_extractor::page_data::ExtractPageDataConfig {
        output_dir: config.storage_dir.clone(),
        link_rewriter: link_rewriter.clone(),
        max_inline_image_size_bytes: config.max_inline_image_size_bytes,
        crawl_rate_rps: config.crawl_rate_rps,
        save_html: config.save_raw_html(),
    };

    let page_data =
        page_extractor::extract_page_data(page.clone(), item.url.clone(), extract_config)
            .await
            .map_err(|e| {
                warn!("Failed to extract page data for {}: {}", item.url, e);
                e
            })?;

    let html_size = page_data.content.len();

    // Save markdown if requested
    if config.save_markdown() {
        let conversion_options = ConversionOptions::default();

        let processed_markdown =
            match convert_html_to_markdown(&page_data.content, &conversion_options).await {
                Ok(md) => md,
                Err(e) => {
                    warn!("Markdown conversion failed: {e}, using html2md fallback");
                    html2md::parse_html(&page_data.content)
                }
            };

        match content_saver::save_markdown_content(
            processed_markdown,
            item.url.clone(),
            config.storage_dir.clone(),
            crate::search::MessagePriority::Normal,
            indexing_sender.clone(),
            config.compress_output,
        )
        .await
        {
            Ok(()) => debug!("Markdown saved for {}", item.url),
            Err(e) => warn!("Failed to save markdown for {}: {}", item.url, e),
        }
    }

    // Save JSON if requested
    if config.save_json() {
        match content_saver::save_page_data(
            Arc::new(page_data.clone()),
            item.url.clone(),
            config.storage_dir.clone(),
        )
        .await
        {
            Ok(()) => debug!("Page data saved for {}", item.url),
            Err(e) => warn!("Failed to save page data for {}: {}", item.url, e),
        }
    }

    // Capture screenshot if requested
    let mut screenshot_captured = false;
    if config.save_screenshots() {
        match page_extractor::capture_screenshot(page.clone(), &item.url, config.storage_dir())
            .await
        {
            Ok(()) => {
                debug!("Screenshot saved for {}", item.url);
                screenshot_captured = true;
            }
            Err(e) => warn!("Failed to save screenshot for {}: {}", item.url, e),
        }
    }

    // Process page links - this needs thread-safe handling
    let links_found = {
        // Create temporary Bloom filter for compatibility with link processor
        let temp_bloom = match Bloom::new_for_fp_rate(10_000_000, 0.01) {
            Ok(b) => b,
            Err(e) => {
                warn!("Failed to create temp Bloom filter: {e}");
                return Ok(item.url);
            }
        };

        let q_snapshot = queue.lock().await.clone();
        let crawl_state = CrawlState {
            queue: q_snapshot,
            visited_urls: temp_bloom,
            max_depth: config.max_depth,
        };

        match process_page_links(page.clone(), item.clone(), crawl_state, &config).await {
            Ok((new_queue, _)) => {
                // Add new discovered links to shared queue
                let mut q = queue.lock().await;
                let mut added = 0;
                for new_item in new_queue {
                    if !q.iter().any(|existing| existing.url == new_item.url) {
                        q.push_back(new_item);
                        added += 1;
                    }
                }
                added
            }
            Err(e) => {
                warn!("Failed to process links for {}: {}", item.url, e);
                0
            }
        }
    };

    // Increment total pages counter
    total_pages.fetch_add(1, Ordering::Relaxed);

    // Record circuit breaker success
    if let Some(ref cb) = circuit_breaker
        && let Ok(domain) = extract_domain(&item.url)
    {
        let mut cb_guard = cb.lock().await;
        cb_guard.record_success(&domain);
    }

    // Publish PageCrawled event
    if let Some(bus) = &event_bus {
        let metadata = PageCrawlMetadata {
            html_size,
            compressed_size: 0,
            links_found,
            links_for_crawling: links_found,
            screenshot_captured,
            processing_duration: page_start.elapsed(),
        };

        let local_path = match crate::content_saver::get_mirror_path_sync(
            &item.url,
            &config.storage_dir,
            "index.md",
        ) {
            Ok(path) => path,
            Err(e) => {
                error!("Failed to compute local path for {}: {}", item.url, e);
                let url_hash = {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    item.url.hash(&mut hasher);
                    hasher.finish()
                };
                config
                    .storage_dir
                    .join("parse-failed")
                    .join(format!("{url_hash}.md"))
            }
        };

        let event = CrawlEvent::page_crawled(
            item.url.clone(),
            local_path,
            u32::from(item.depth),
            metadata,
        );

        if let Err(e) = bus.publish(event).await {
            warn!(
                "Failed to publish PageCrawled event for {}: {}",
                item.url, e
            );
        }
    }

    Ok(item.url)
}
