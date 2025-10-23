//! Browser lifecycle manager for web search
//!
//! Manages a shared chromiumoxide browser instance following the same pattern
//! as `CrawlSessionManager`. Browser is launched on first search and reused for
//! subsequent searches to improve performance.

use anyhow::Result;
use std::sync::{Arc, OnceLock};
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
/// Uses `Arc<Mutex<Option<BrowserWrapper>>>` for async-safe access.
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

    /// Get or launch the shared browser instance
    ///
    /// Uses double-check locking to prevent race conditions during first browser launch.
    /// Multiple concurrent calls will not launch multiple browsers.
    ///
    /// # Performance
    /// - First call: ~2-3s (launches browser)
    /// - Subsequent calls: <1ms (fast path, just clones Arc)
    ///
    /// # Returns
    /// Reference to the `BrowserWrapper` for creating pages
    pub async fn get_or_launch(&self) -> Result<Arc<Mutex<Option<BrowserWrapper>>>> {
        // Fast path - check if already initialized (no init lock needed)
        {
            let browser_lock = self.browser.lock().await;
            if browser_lock.is_some() {
                return Ok(self.browser.clone());
            }
        }
        
        // Slow path - use separate init lock to serialize browser creation
        // This prevents race condition where multiple tasks see None and all launch browsers
        static INIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let init_lock = INIT_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = init_lock.lock().await;
        
        // Double-check after acquiring init lock (another task may have initialized)
        {
            let browser_lock = self.browser.lock().await;
            if browser_lock.is_some() {
                return Ok(self.browser.clone());
            }
        }
        
        // Now safe to launch - only one task can be here at a time
        info!("Launching browser for first web search (will be reused)");
        let (browser, handler) = launch_browser().await?;
        let wrapper = BrowserWrapper::new(browser, handler);
        
        let mut browser_lock = self.browser.lock().await;
        *browser_lock = Some(wrapper);
        
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
        let mut browser_lock = self.browser.lock().await;
        
        if let Some(mut wrapper) = browser_lock.take() {
            info!("Shutting down web search browser");
            
            // CRITICAL: Must call browser.close() AND wait() to prevent warning
            // 1. Close the browser
            if let Err(e) = wrapper.browser_mut().close().await {
                tracing::warn!("Failed to close browser cleanly: {}", e);
            }
            
            // 2. Wait for process to fully exit (prevents "not closed manually" warning)
            if let Err(e) = wrapper.browser_mut().wait().await {
                tracing::warn!("Failed to wait for browser exit: {}", e);
            }
            
            // Now drop the wrapper (calls handler.abort())
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
