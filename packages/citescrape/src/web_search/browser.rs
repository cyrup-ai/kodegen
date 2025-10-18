//! Browser lifecycle management for web search
//!
//! Handles launching and managing chromiumoxide browser instances with
//! stealth configuration to avoid bot detection.

use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use futures::StreamExt;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::{self, JoinHandle};
use tracing::{error, info};

/// Wrapper for Browser and its event handler task
///
/// Ensures handler is properly cleaned up when browser is dropped.
/// Handler MUST be aborted to prevent it running indefinitely after
/// browser is closed.
pub(crate) struct BrowserWrapper {
    browser: Browser,
    handler: JoinHandle<()>,
}

impl BrowserWrapper {
    pub(crate) fn new(browser: Browser, handler: JoinHandle<()>) -> Self {
        Self { browser, handler }
    }

    /// Get reference to inner browser
    pub(crate) fn browser(&self) -> &Browser {
        &self.browser
    }
}

impl Drop for BrowserWrapper {
    fn drop(&mut self) {
        info!("Dropping BrowserWrapper - aborting handler task");
        self.handler.abort();
        // Handler will be awaited/cleaned up by tokio runtime
    }
}

/// Global browser instance, initialized on first search
///
/// Uses Arc for proper lifecycle management. The browser is shared across
/// all web searches and cleaned up when the last reference is dropped.
static GLOBAL_BROWSER: OnceCell<Arc<BrowserWrapper>> = OnceCell::new();

/// Launch a new browser instance with stealth configuration
///
/// Returns a tuple of (Browser, JoinHandle) where the JoinHandle tracks the
/// browser event handler task. The handler must be properly awaited when
/// closing the browser to prevent resource leaks.
///
/// # Based on
/// - packages/citescrape/src/crawl_engine/core.rs:174-191 (browser launch pattern)
/// - packages/citescrape/src/google_search.rs:88-98 (stealth args)
pub async fn launch_browser() -> Result<(Browser, JoinHandle<()>)> {
    let browser_config = BrowserConfig::builder()
        .request_timeout(Duration::from_secs(30))
        .window_size(1920, 1080)
        .arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .arg("--disable-blink-features=AutomationControlled")
        .arg("--exclude-switches=enable-automation")
        .arg("--disable-infobars")
        .arg("--disable-dev-shm-usage")
        .arg("--disable-gpu")
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build browser config: {e}"))?;

    info!("Launching browser with stealth config");
    let (browser, mut handler) = Browser::launch(browser_config)
        .await
        .context("Failed to launch browser")?;

    // Spawn handler task to process browser events with tracked lifetime
    let handler_task = task::spawn(async move {
        while let Some(h) = handler.next().await {
            if let Err(e) = h {
                error!("Browser handler error: {:?}", e);
            }
        }
        info!("Browser handler task terminated");
    });

    Ok((browser, handler_task))
}

/// Create a blank page for stealth injection
///
/// Creates a page with about:blank URL, which is required for proper
/// kromekover stealth injection timing. The page must be blank before
/// stealth features are applied, then navigation to the target URL occurs.
///
/// # Arguments
/// * `wrapper` - BrowserWrapper containing the browser instance
///
/// # Returns
/// A blank Page instance ready for stealth enhancement
///
/// # Based on
/// - packages/citescrape/src/crawl_engine/core.rs:231-237 (about:blank pattern)
pub async fn create_blank_page(wrapper: &BrowserWrapper) -> Result<Page> {
    let page = wrapper
        .browser()
        .new_page("about:blank")
        .await
        .context("Failed to create blank page")?;

    info!("Created blank page for stealth injection");
    Ok(page)
}

/// Get or launch the shared browser instance
///
/// On first call: launches new browser (2-3s)
/// Subsequent calls: returns Arc clone (instant, cheap)
///
/// Returns Arc<BrowserWrapper> which caller can hold across await points.
/// When all Arc references are dropped, browser is automatically cleaned up.
///
/// # Returns
/// Arc<BrowserWrapper> containing the browser and its handler task
///
/// # Errors
/// Returns error if browser launch fails on first call
pub async fn get_or_launch_browser() -> Result<Arc<BrowserWrapper>> {
    // Fast path - clone Arc (cheap, just increments refcount)
    if let Some(browser_wrapper) = GLOBAL_BROWSER.get() {
        return Ok(Arc::clone(browser_wrapper));
    }

    // Slow path - use mutex to serialize browser creation
    use tokio::sync::Mutex;
    static INIT_LOCK: Mutex<()> = Mutex::const_new(());
    let _guard = INIT_LOCK.lock().await;

    // Double-check after acquiring lock
    if let Some(browser_wrapper) = GLOBAL_BROWSER.get() {
        return Ok(Arc::clone(browser_wrapper));
    }

    // Launch browser with proper lifecycle management
    info!("Launching browser for first time (will be reused for all searches)");
    let (browser, handler) = launch_browser().await?;
    let wrapper = Arc::new(BrowserWrapper::new(browser, handler));

    // Store in global (this will always succeed due to lock)
    GLOBAL_BROWSER
        .set(Arc::clone(&wrapper))
        .map_err(|_| anyhow::anyhow!("Failed to set GLOBAL_BROWSER"))?;

    Ok(wrapper)
}
