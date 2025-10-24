//! Browser infrastructure for launching and managing Chrome instances
//!
//! Based on production-tested code from packages/tools-citescrape

mod setup;
mod wrapper;

pub use setup::{find_browser_executable, download_managed_browser};
pub use wrapper::{BrowserWrapper, launch_browser};

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
