//! Browser lifecycle management for web search
//!
//! Handles launching and managing chromiumoxide browser instances with
//! stealth configuration to avoid bot detection.

use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfigBuilder};
use chromiumoxide::page::Page;
use futures::StreamExt;
use std::time::Duration;
use tokio::task::{self, JoinHandle};
use tracing::info;

/// Wrapper for Browser and its event handler task
///
/// Ensures handler is properly cleaned up when browser is dropped.
/// Handler MUST be aborted to prevent it running indefinitely after
/// browser is closed.
pub struct BrowserWrapper {
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

    /// Get mutable reference to inner browser
    pub(crate) fn browser_mut(&mut self) -> &mut Browser {
        &mut self.browser
    }
}

impl Drop for BrowserWrapper {
    fn drop(&mut self) {
        info!("Dropping BrowserWrapper - aborting handler task");
        self.handler.abort();
        // Handler will be awaited/cleaned up by tokio runtime
        // Browser::drop() will automatically kill the Chrome process
        // (it logs a warning but prevents zombie processes)
    }
}



/// Launch a new browser instance with stealth configuration
///
/// Returns a tuple of (Browser, `JoinHandle`) where the `JoinHandle` tracks the
/// REAL browser event handler task. The handler must be aborted when
/// closing the browser to prevent zombie processes.
///
/// This function calls chromiumoxide `Browser::launch()` directly and properly
/// tracks the handler, unlike `browser_setup::launch_browser()` which spawns
/// a detached handler task that cannot be stopped.
///
/// # Handler Lifecycle
/// The returned `JoinHandle` MUST be aborted when done to stop the browser process.
/// `BrowserWrapper::drop()` handles this automatically.
pub async fn launch_browser() -> Result<(Browser, JoinHandle<()>)> {
    info!("Launching browser for web search");

    // Find or download Chrome executable
    let chrome_path = match crate::browser::find_browser_executable().await {
        Ok(path) => path,
        Err(_) => crate::browser::download_managed_browser().await?,
    };
    
    // Create unique temp directory for this browser instance
    let user_data_dir = std::env::temp_dir()
        .join(format!("enigo_chrome_{}", std::process::id()));
    
    std::fs::create_dir_all(&user_data_dir)
        .context("Failed to create user data directory")?;
    
    // Build browser config with stealth settings
    let browser_config = BrowserConfigBuilder::default()
        .request_timeout(Duration::from_secs(30))
        .window_size(1920, 1080)
        .user_data_dir(user_data_dir)
        .chrome_executable(chrome_path)
        .headless_mode(chromiumoxide::browser::HeadlessMode::default())
        // Stealth mode arguments
        .arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .arg("--disable-blink-features=AutomationControlled")
        .arg("--disable-infobars")
        .arg("--disable-notifications")
        .arg("--disable-print-preview")
        .arg("--disable-desktop-notifications")
        .arg("--disable-software-rasterizer")
        .arg("--disable-web-security")
        .arg("--disable-features=IsolateOrigins,site-per-process")
        .arg("--disable-setuid-sandbox")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--no-sandbox")
        .arg("--ignore-certificate-errors")
        .arg("--enable-features=NetworkService,NetworkServiceInProcess")
        .arg("--disable-extensions")
        .arg("--disable-popup-blocking")
        .arg("--disable-background-networking")
        .arg("--disable-background-timer-throttling")
        .arg("--disable-backgrounding-occluded-windows")
        .arg("--disable-breakpad")
        .arg("--disable-component-extensions-with-background-pages")
        .arg("--disable-features=TranslateUI")
        .arg("--disable-hang-monitor")
        .arg("--disable-ipc-flooding-protection")
        .arg("--disable-prompt-on-repost")
        .arg("--metrics-recording-only")
        .arg("--password-store=basic")
        .arg("--use-mock-keychain")
        .arg("--hide-scrollbars")
        .arg("--mute-audio")
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build browser config: {e}"))?;
    
    info!("Launching browser with config");
    
    // Launch browser and get REAL handler (not a dummy)
    let (browser, mut handler) = Browser::launch(browser_config)
        .await
        .context("Failed to launch browser")?;
    
    // Spawn handler with TRACKED JoinHandle (this is the critical fix)
    let handler_task = task::spawn(async move {
        while let Some(event) = handler.next().await {
            if let Err(e) = event {
                tracing::error!("Browser handler error: {:?}", e);
            }
        }
        info!("Browser event handler task completed");
    });

    Ok((browser, handler_task))
}

/// Create a blank page for stealth injection
///
/// Creates a page with about:blank URL, which is required for proper
/// stealth injection timing. The page must be blank before
/// stealth features are applied, then navigation to the target URL occurs.
///
/// # Arguments
/// * `wrapper` - `BrowserWrapper` containing the browser instance
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
