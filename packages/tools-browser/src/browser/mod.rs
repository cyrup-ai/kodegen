//! Browser infrastructure for launching and managing Chrome instances
//!
//! Based on production-tested code from packages/tools-citescrape

mod wrapper;

pub use crate::browser_setup::{download_managed_browser, find_browser_executable};
pub use wrapper::{BrowserWrapper, create_blank_page, get_current_page, launch_browser};

use chromiumoxide::page::Page;
use std::sync::Arc;

/// Browser context wrapper for legacy code compatibility
///
/// NOTE: In hot path, prefer using existing tools via MCP client.
/// This is primarily for DeepResearch search result extraction where
/// direct browser access is needed for parsing search engine results.
pub struct BrowserContext {
    manager: Arc<crate::manager::BrowserManager>,
}

impl BrowserContext {
    pub fn new(manager: Arc<crate::manager::BrowserManager>) -> Self {
        Self { manager }
    }

    /// Get current page - use sparingly, prefer MCP tools
    pub async fn get_current_page(&self) -> BrowserResult<Page> {
        let browser_arc = self
            .manager
            .get_or_launch()
            .await
            .map_err(|e| BrowserError::LaunchFailed(e.to_string()))?;
        let browser_guard = browser_arc.lock().await;
        let wrapper = browser_guard
            .as_ref()
            .ok_or_else(|| BrowserError::PageCreationFailed("Browser not available".into()))?;

        let browser = wrapper.browser();
        let pages = browser
            .pages()
            .await
            .map_err(|e| BrowserError::PageCreationFailed(e.to_string()))?;

        if let Some(page) = pages.first() {
            Ok(page.clone())
        } else {
            browser
                .new_page("about:blank")
                .await
                .map_err(|e| BrowserError::PageCreationFailed(e.to_string()))
        }
    }

    /// Take screenshot - prefer browser_screenshot tool via MCP
    pub async fn screenshot(&self) -> BrowserResult<Vec<u8>> {
        let page = self.get_current_page().await?;
        page.screenshot(chromiumoxide::page::ScreenshotParams::builder().build())
            .await
            .map_err(|e| BrowserError::NavigationFailed(e.to_string()))
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrowserError {
    #[error("Failed to find browser executable: {0}")]
    NotFound(String),

    #[error("Failed to launch browser: {0}")]
    LaunchFailed(String),

    #[error("Failed to create page: {0}")]
    PageCreationFailed(String),

    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    #[error("IO error: {0}")]
    IoError(String),
}

pub type BrowserResult<T> = Result<T, BrowserError>;
