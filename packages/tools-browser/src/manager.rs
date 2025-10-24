//! Browser manager for coordinating browser instances and pages

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::browser::{Browser, BrowserConfig, BrowserContext, BrowserContextConfig, BrowserResult};

/// Manager for browser instances and contexts
#[derive(Clone)]
pub struct BrowserManager {
    browser: Arc<Mutex<Option<Browser>>>,
    context: Arc<Mutex<Option<BrowserContext>>>,
    config: BrowserConfig,
}

impl BrowserManager {
    /// Create a new browser manager
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            browser: Arc::new(Mutex::new(None)),
            context: Arc::new(Mutex::new(None)),
            config,
        }
    }

    /// Get or create browser instance
    pub async fn get_or_create_browser(&self) -> BrowserResult<Browser> {
        let mut browser_guard = self.browser.lock().await;
        if browser_guard.is_none() {
            let browser = Browser::new(self.config.clone()).await?;
            *browser_guard = Some(browser);
        }
        Ok(browser_guard.as_ref().unwrap().clone())
    }

    /// Get or create browser context
    pub async fn get_or_create_context(&self) -> BrowserResult<BrowserContext> {
        let browser = self.get_or_create_browser().await?;
        let mut context_guard = self.context.lock().await;
        if context_guard.is_none() {
            let context = browser.new_context(BrowserContextConfig::default()).await?;
            *context_guard = Some(context);
        }
        Ok(context_guard.as_ref().unwrap().clone())
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new(BrowserConfig::default())
    }
}
