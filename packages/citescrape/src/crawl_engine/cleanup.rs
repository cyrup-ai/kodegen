//! Browser and resource cleanup functionality
//!
//! This module handles cleanup tasks after crawling is complete.

use anyhow::Result;
use chromiumoxide::Browser;
use log::{debug, info, warn};
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

/// Result of cleanup operations
#[derive(Debug, Clone)]
pub enum CleanupResult {
    /// All cleanup operations succeeded
    Success,
    /// Some cleanup operations failed, with error details
    PartialFailure(Vec<String>),
}

/// Clean up browser and Chrome data directory
pub fn cleanup_browser_and_data(
    mut browser: Browser,
    chrome_data_dir_path: std::path::PathBuf,
    on_result: impl FnOnce(Result<CleanupResult>) + Send + 'static,
) -> crate::runtime::AsyncTask<()> {
    use crate::runtime::spawn_async;
    
    spawn_async(async move {
        let result = async {
            let mut errors = Vec::new();

            debug!(target: "citescrape::cleanup", "Closing browser");
            if let Err(e) = browser.close().await {
                warn!(target: "citescrape::cleanup", "Failed to close browser: {}", e);
                errors.push(format!("Browser close failed: {}", e));
            } else {
                debug!(target: "citescrape::cleanup", "Browser closed successfully");
            }

            debug!(target: "citescrape::cleanup", "Cleaning up Chrome data directory");
            if let Err(e) = std::fs::remove_dir_all(&chrome_data_dir_path) {
                warn!(target: "citescrape::cleanup", "Failed to clean up Chrome data directory: {}", e);
                errors.push(format!("Directory cleanup failed: {}", e));
            } else {
                debug!(target: "citescrape::cleanup", "Chrome data directory cleaned up successfully");
            }

            if errors.is_empty() {
                Ok(CleanupResult::Success)
            } else {
                Ok(CleanupResult::PartialFailure(errors))
            }
        }.await;
        
        on_result(result);
    })
}

/// Finalize logging and wait for log handler
pub fn finalize_logging(
    tx: Sender<String>,
    log_handle: JoinHandle<()>,
    on_result: impl FnOnce(Result<CleanupResult>) + Send + 'static,
) -> crate::runtime::AsyncTask<()> {
    use crate::runtime::spawn_async;
    
    spawn_async(async move {
        let result = async {
            let mut errors = Vec::new();

            // Drop the sender to close the logging channel
            drop(tx);

            // Wait for the logging task to complete
            if let Err(e) = log_handle.await {
                warn!(target: "citescrape::cleanup", "Error waiting for log handler: {}", e);
                errors.push(format!("Log handler failed: {}", e));
            }

            info!(target: "citescrape::crawl", "Crawl completed successfully");

            if errors.is_empty() {
                Ok(CleanupResult::Success)
            } else {
                Ok(CleanupResult::PartialFailure(errors))
            }
        }.await;
        
        on_result(result);
    })
}