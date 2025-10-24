//! Browser instance manager for resource-efficient browser sharing
//!
//! Ensures only one browser runs at a time, shared across all tools.
//!
//! # Architecture
//! 
//! Uses `Arc<Mutex<Option<BrowserWrapper>>>` pattern from citescrape for:
//! - Thread-safe lazy initialization
//! - Automatic browser launch on first use
//! - Shared access from multiple tools
//! - Proper cleanup on shutdown
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
use tokio::sync::Mutex;
use tracing::info;

use crate::browser::{BrowserWrapper, launch_browser};

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
    browser: Arc<Mutex<Option<BrowserWrapper>>>,
}

impl BrowserManager {
    /// Create a new BrowserManager (no browser launched yet)
    ///
    /// Browser will be lazy-loaded on first `get_or_launch()` call.
    #[must_use]
    pub fn new() -> Self {
        Self {
            browser: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Get or launch the shared browser instance
    ///
    /// Uses double-check locking with OnceLock to prevent race conditions
    /// during first browser launch. Multiple concurrent calls will not
    /// launch multiple browsers.
    ///
    /// # Performance
    /// - First call: ~2-3s (launches browser)
    /// - Subsequent calls: <1ms (fast path, just clones Arc)
    ///
    /// # Double-Check Pattern
    ///
    /// ```text
    /// 1. Fast path: Check if browser exists (no init lock)
    ///    → If exists, return immediately
    ///
    /// 2. Slow path: Acquire init lock to serialize launch
    ///    → Prevents race where N threads all see None and launch N browsers
    ///
    /// 3. Double-check: Check again after acquiring init lock
    ///    → Another thread may have initialized while we waited
    ///
    /// 4. Launch: Now safe - only one thread can be here
    /// ```
    ///
    /// # Returns
    /// Arc to the browser Mutex - caller locks it to access BrowserWrapper
    ///
    /// # Example
    /// ```rust
    /// let manager = BrowserManager::new();
    /// let browser_arc = manager.get_or_launch().await?;
    /// let browser_guard = browser_arc.lock().await;
    /// if let Some(wrapper) = browser_guard.as_ref() {
    ///     let page = wrapper.browser().new_page("https://example.com").await?;
    /// }
    /// ```
    ///
    /// Based on: packages/tools-citescrape/src/web_search/manager.rs:46-79
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
        info!("Launching browser for first use (will be reused)");
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
        let mut browser_lock = self.browser.lock().await;
        
        if let Some(mut wrapper) = browser_lock.take() {
            info!("Shutting down browser");
            
            // 1. Close current page (prevents "page not closed" warning)
            if let Err(e) = wrapper.close_current_page().await {
                tracing::warn!("Failed to close page cleanly: {}", e);
            }
            
            // 2. Close the browser
            if let Err(e) = wrapper.browser_mut().close().await {
                tracing::warn!("Failed to close browser cleanly: {}", e);
            }
            
            // 3. Wait for process to fully exit
            if let Err(e) = wrapper.browser_mut().wait().await {
                tracing::warn!("Failed to wait for browser exit: {}", e);
            }
            
            drop(wrapper);
        }
        
        Ok(())
    }
    
    /// Check if browser is currently running
    ///
    /// Non-blocking check of browser state.
    pub async fn is_browser_running(&self) -> bool {
        self.browser.lock().await.is_some()
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
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
