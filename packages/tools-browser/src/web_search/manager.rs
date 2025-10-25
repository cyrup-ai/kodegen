//! Browser lifecycle manager for web search
//!
//! Manages a shared chromiumoxide browser instance following the same pattern
//! as `CrawlSessionManager`. Browser is launched on first search and reused for
//! subsequent searches to improve performance.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
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
/// Uses `Arc<Mutex<Option<BrowserWrapper>>>` for async-safe access
/// with automatic health checking and recovery.
#[derive(Clone)]
pub struct BrowserManager {
    browser: Arc<Mutex<Option<BrowserWrapper>>>,
}

impl BrowserManager {
    /// Create a new browser manager
    ///
    /// Browser is NOT launched yet - it will be lazy-loaded on first search.
    #[must_use]
    pub fn new() -> Self {
        Self {
            browser: Arc::new(Mutex::new(None)),
        }
    }

    /// Get or launch the shared browser instance with health checking and auto-recovery
    ///
    /// # Health Check and Recovery Flow
    /// 1. Lock browser mutex
    /// 2. If browser exists, check health via version() CDP command
    /// 3. If unhealthy, close crashed browser and remove from cache
    /// 4. If no browser or was unhealthy, launch new instance
    /// 5. Return healthy browser
    ///
    /// # First Call
    /// - ~2-3s (launches browser)
    ///
    /// # Subsequent Calls (healthy browser)
    /// - <1ms (mutex lock + Arc clone)
    ///
    /// # Recovery from Crash
    /// - ~2-3s (detects crash + closes + re-launches)
    /// - Automatic, no user intervention required
    pub async fn get_or_launch(&self) -> Result<Arc<Mutex<Option<BrowserWrapper>>>> {
        let mut guard = self.browser.lock().await;

        // Health check: if browser exists, verify it's alive
        if let Some(wrapper) = guard.as_ref() {
            match wrapper.browser().version().await {
                Ok(_) => {
                    tracing::debug!("Browser health check passed, reusing existing browser");
                    // Browser is healthy, return it
                    drop(guard); // Release lock
                    return Ok(self.browser.clone());
                }
                Err(e) => {
                    tracing::warn!("Browser health check failed: {}. Triggering recovery...", e);

                    // Take ownership and clean up crashed browser
                    if let Some(mut crashed_wrapper) = guard.take() {
                        // Best-effort cleanup (may fail if process already dead)
                        let _ = crashed_wrapper.browser_mut().close().await;
                        let _ = crashed_wrapper.browser_mut().wait().await;
                        crashed_wrapper.cleanup_temp_dir();
                    }

                    tracing::info!("Crashed browser cleaned up, launching new instance");
                }
            }
        }

        // No browser exists or previous one crashed - launch new one
        tracing::info!("Launching browser (first time or after recovery)");
        let (browser, handler, user_data_dir) = launch_browser().await?;
        let wrapper = BrowserWrapper::new(browser, handler, user_data_dir);
        *guard = Some(wrapper);
        drop(guard);

        Ok(self.browser.clone())
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
        let mut guard = self.browser.lock().await;

        if let Some(mut wrapper) = guard.take() {
            info!("Shutting down web search browser");

            // Close browser gracefully
            if let Err(e) = wrapper.browser_mut().close().await {
                tracing::warn!("Failed to close browser cleanly: {}", e);
            }

            // Wait for process to fully exit
            if let Err(e) = wrapper.browser_mut().wait().await {
                tracing::warn!("Failed to wait for browser exit: {}", e);
            }

            // Cleanup temp directory
            wrapper.cleanup_temp_dir();

            drop(wrapper);
        }

        Ok(())
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
    }
}
