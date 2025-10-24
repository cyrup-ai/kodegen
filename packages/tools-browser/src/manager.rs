//! Browser instance manager for resource-efficient browser sharing
//!
//! Ensures only one browser runs at a time, shared across all tools.
//!
//! # Architecture
//!
//! Uses `Arc<OnceCell<Arc<Mutex<Option<BrowserWrapper>>>>>` pattern:
//! - Thread-safe lazy initialization via OnceCell
//! - Automatic browser launch on first use
//! - Shared access from multiple tools
//! - Proper cleanup on shutdown
//! - Atomic initialization without race conditions
//!
//! # Async Lock Requirements
//!
//! CRITICAL: Must use `tokio::sync::Mutex`, NOT `parking_lot::RwLock`
//! - Browser operations are async (`.await` everywhere)
//! - Cannot hold sync locks across `.await` points
//! - tokio::sync::Mutex is Send-safe for async contexts
//!
//! Reference: packages/tools-citescrape/src/web_search/manager.rs

use anyhow::Result;
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, OnceCell};
use tracing::info;

use crate::browser::{BrowserWrapper, launch_browser};

// Global singleton instance
static GLOBAL_MANAGER: OnceLock<Arc<BrowserManager>> = OnceLock::new();

/// Singleton manager for browser instances
///
/// Manages browser lifecycle to ensure:
/// - Only one browser instance exists at a time (lazy-loaded)
/// - Automatic launch on first use (~2-3s first call, instant after)
/// - Thread-safe access from multiple tools
/// - Proper cleanup when dropped or shutdown
///
/// # Performance Characteristics
///
/// - First `get_or_launch()`: ~2-3 seconds (launches Chrome)
/// - Subsequent calls: <1ms (returns Arc clone)
/// - Memory: ~150MB per browser instance (Chrome process)
///
/// # Pattern Source
///
/// Based on: packages/tools-citescrape/src/web_search/manager.rs:14-122
pub struct BrowserManager {
    browser: Arc<OnceCell<Arc<Mutex<Option<BrowserWrapper>>>>>,
}

impl BrowserManager {
    /// Get the global singleton BrowserManager instance
    ///
    /// This ensures only one browser instance runs process-wide.
    /// All tools should use this method instead of creating their own managers.
    ///
    /// # Thread Safety
    /// Uses `OnceLock` for atomic initialization - safe to call from multiple threads.
    /// First caller initializes, concurrent callers block until initialization completes.
    ///
    /// # Performance
    /// - First call: ~50ns (creates BrowserManager struct)
    /// - Subsequent calls: ~5ns (atomic pointer load)
    /// - Browser launch still lazy on first `get_or_launch()` call (~2-3s)
    ///
    /// # Example
    /// ```rust
    /// let manager = BrowserManager::global();
    /// let browser_arc = manager.get_or_launch().await?;
    /// ```
    #[must_use]
    pub fn global() -> Arc<BrowserManager> {
        GLOBAL_MANAGER
            .get_or_init(|| Arc::new(BrowserManager::new()))
            .clone()
    }

    /// Create a new BrowserManager (private - use global() instead)
    ///
    /// Browser will be lazy-loaded on first `get_or_launch()` call.
    ///
    /// This is now private to prevent accidental creation of multiple managers.
    /// External code should use `BrowserManager::global()`.
    fn new() -> Self {
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
    /// Arc to the browser Mutex - caller locks it to access BrowserWrapper
    ///
    /// # Example
    /// ```rust
    /// let manager = BrowserManager::global();
    /// let browser_arc = manager.get_or_launch().await?;
    /// let browser_guard = browser_arc.lock().await;
    /// if let Some(wrapper) = browser_guard.as_ref() {
    ///     let page = wrapper.browser().new_page("https://example.com").await?;
    /// }
    /// ```
    pub async fn get_or_launch(&self) -> Result<Arc<Mutex<Option<BrowserWrapper>>>> {
        let browser_arc = self
            .browser
            .get_or_try_init(|| async {
                info!("Launching browser for first use (will be reused)");
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
    /// # Critical Implementation Note
    ///
    /// We must call BOTH:
    /// 1. `browser.close().await` - Sends close command to Chrome
    /// 2. `browser.wait().await` - Waits for process to fully exit
    ///
    /// WHY: `BrowserWrapper::drop()` only aborts the handler task.
    /// It does NOT close the browser process. Without explicit close(),
    /// Chrome process becomes a zombie and logs warnings.
    ///
    /// # Example from citescrape
    /// ```rust
    /// // packages/tools-citescrape/src/web_search/manager.rs:99-114
    /// if let Some(mut wrapper) = browser_lock.take() {
    ///     info!("Shutting down browser");
    ///     
    ///     // 1. Close the browser
    ///     if let Err(e) = wrapper.browser_mut().close().await {
    ///         tracing::warn!("Failed to close browser cleanly: {}", e);
    ///     }
    ///     
    ///     // 2. Wait for process to fully exit
    ///     if let Err(e) = wrapper.browser_mut().wait().await {
    ///         tracing::warn!("Failed to wait for browser exit: {}", e);
    ///     }
    ///     
    ///     // 3. Now drop the wrapper (calls handler.abort())
    ///     drop(wrapper);
    /// }
    /// ```
    ///
    /// Based on: packages/tools-citescrape/src/web_search/manager.rs:88-122
    pub async fn shutdown(&self) -> Result<()> {
        // Check if browser was ever initialized
        if let Some(browser_arc) = self.browser.get() {
            let mut browser_lock = browser_arc.lock().await;

            if let Some(mut wrapper) = browser_lock.take() {
                info!("Shutting down browser");

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

    /// Check if browser is currently running
    ///
    /// Non-blocking check of browser state.
    pub async fn is_browser_running(&self) -> bool {
        if let Some(browser_arc) = self.browser.get() {
            browser_arc.lock().await.is_some()
        } else {
            false
        }
    }
}

impl Drop for BrowserManager {
    fn drop(&mut self) {
        // Cleanup happens via BrowserWrapper::drop() automatically
        // However, this is NOT a clean shutdown - it only aborts the handler
        // For clean shutdown, call shutdown().await before dropping
        info!("BrowserManager dropping - browser will be cleaned up");
    }
}
