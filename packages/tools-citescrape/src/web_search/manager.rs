//! Browser lifecycle manager for web search
//!
//! Manages a shared chromiumoxide browser instance following the same pattern
//! as `CrawlSessionManager`. Browser is launched on first search and reused for
//! subsequent searches to improve performance.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};
use tracing::info;

use super::browser::{BrowserWrapper, launch_browser};

/// Manager for shared browser instance used by web searches
///
/// Pattern based on [`CrawlSessionManager`](../../mcp/manager.rs)
/// for consistency with existing codebase.
///
/// # Lifecycle
/// - Browser NOT launched on manager creation (lazy initialization)
/// - First `get_or_launch()` call launches browser (~2-3s)
/// - Subsequent calls return existing browser (instant)
/// - `shutdown()` explicitly closes browser (called on server shutdown)
///
/// # Thread Safety
/// Uses `Arc<OnceCell<Arc<Mutex<Option<BrowserWrapper>>>>>` for async-safe access
/// with atomic initialization via OnceCell.
#[derive(Clone)]
pub struct BrowserManager {
    browser: Arc<OnceCell<Arc<Mutex<Option<BrowserWrapper>>>>>,
}

impl BrowserManager {
    /// Create a new browser manager
    ///
    /// Browser is NOT launched yet - it will be lazy-loaded on first search.
    #[must_use]
    pub fn new() -> Self {
        Self {
            browser: Arc::new(OnceCell::new()),
        }
    }

    /// Get or launch the shared browser instance
    ///
    /// Uses OnceCell for atomic async initialization to prevent race conditions
    /// during first browser launch. Multiple concurrent calls will not
    /// launch multiple browsers.
    ///
    /// # Performance
    /// - First call: ~2-3s (launches browser)
    /// - Subsequent calls: <1ms (atomic pointer load, no locks)
    ///
    /// # OnceCell Pattern
    ///
    /// OnceCell ensures exactly-once async initialization:
    /// - First caller executes initialization closure
    /// - Concurrent callers await the same initialization
    /// - All callers receive the same initialized value
    /// - No race windows or thundering herd behavior
    ///
    /// # Returns
    /// Reference to the `BrowserWrapper` for creating pages
    pub async fn get_or_launch(&self) -> Result<Arc<Mutex<Option<BrowserWrapper>>>> {
        let browser_arc = self
            .browser
            .get_or_try_init(|| async {
                info!("Launching browser for first web search (will be reused)");
                let (browser, handler, user_data_dir) = launch_browser().await?;
                let wrapper = BrowserWrapper::new(browser, handler, user_data_dir);
                Ok::<_, anyhow::Error>(Arc::new(Mutex::new(Some(wrapper))))
            })
            .await?;

        Ok(browser_arc.clone())
    }

    /// Shutdown the browser if running
    ///
    /// Explicitly closes the browser process and cleans up resources.
    /// Safe to call multiple times (subsequent calls are no-ops).
    ///
    /// # Implementation Note
    /// We must call `browser.close().await` explicitly because
    /// `BrowserWrapper::drop()` only aborts the handler, it does NOT
    /// close the browser process. See [`cleanup_browser_and_data`](../../crawl_engine/cleanup.rs)
    /// for the pattern.
    pub async fn shutdown(&self) -> Result<()> {
        // Check if browser was ever initialized
        if let Some(browser_arc) = self.browser.get() {
            let mut browser_lock = browser_arc.lock().await;

            if let Some(mut wrapper) = browser_lock.take() {
                info!("Shutting down web search browser");

                // CRITICAL: Must call browser.close() AND wait() to prevent warning
                // 1. Close the browser
                if let Err(e) = wrapper.browser_mut().close().await {
                    tracing::warn!("Failed to close browser cleanly: {}", e);
                }

                // 2. Wait for process to fully exit (CRITICAL - releases file handles)
                if let Err(e) = wrapper.browser_mut().wait().await {
                    tracing::warn!("Failed to wait for browser exit: {}", e);
                }

                // 3. Cleanup temp directory
                wrapper.cleanup_temp_dir();

                // 4. Drop wrapper (aborts handler)
                drop(wrapper);
            }
        }

        Ok(())
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
    }
}
