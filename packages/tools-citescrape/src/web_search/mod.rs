//! Web search functionality using browser automation
//!
//! This module provides a clean API for performing web searches and extracting
//! results. It orchestrates browser management, search execution, and result
//! extraction with automatic retry logic.
//!
//! # Architecture
//! - `types` - Data structures and constants
//! - `browser` - Browser lifecycle management
//! - `search` - Search execution and result extraction
//!
//! # Usage Patterns
//!
//! ## Standalone Scripts
//! ```no_run
//! use kodegen_citescrape::web_search;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let results = web_search::search("rust programming").await?;
//!     println!("Found {} results", results.results.len());
//!     
//!     // Clean up browser before exit
//!     web_search::shutdown_standalone().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## MCP Tools (Managed Lifecycle)
//! ```no_run
//! use kodegen_citescrape::web_search::{BrowserManager, search_with_manager};
//!
//! async fn tool_execute(manager: &BrowserManager) -> anyhow::Result<()> {
//!     let results = search_with_manager(manager, "query").await?;
//!     // Manager is shut down by tool_registry on server shutdown
//!     Ok(())
//! }
//! ```

mod browser;
mod manager;
mod search;
mod types;

// Re-export public types
pub use manager::BrowserManager;
pub use types::{MAX_RESULTS, MAX_RETRIES, SearchResult, SearchResults};

use anyhow::Result;
use once_cell::sync::OnceCell;
use tracing::info;

/// Standalone browser manager for convenience API
///
/// Used by `search()` and `shutdown_standalone()` functions.
/// MCP tools should NOT use this - they get a manager from `tool_registry`.
static STANDALONE_MANAGER: OnceCell<BrowserManager> = OnceCell::new();

/// Perform web search using provided `BrowserManager`
///
/// This is the function used by MCP tools. The manager is passed in from
/// the server's tool registry, allowing proper lifecycle management.
///
/// # Arguments
/// * `browser_manager` - Shared browser manager from tool registry
/// * `query` - Search query string
///
/// # Implementation
/// Uses manager instead of global static for browser access.
pub async fn search_with_manager(
    browser_manager: &BrowserManager,
    query: impl Into<String>,
) -> Result<SearchResults> {
    let query = query.into();
    info!("Starting web search for query: {}", query);

    // Get browser from manager (NOT global static)
    let browser_arc = browser_manager.get_or_launch().await?;
    let browser_lock = browser_arc.lock().await;

    let browser_wrapper = browser_lock
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Browser not available"))?;

    // Create fresh page for this search
    let page = browser::create_blank_page(browser_wrapper).await?;

    // Release lock before performing search
    drop(browser_lock);

    // Perform search with retry logic (unchanged from current implementation)
    let results = search::retry_with_backoff(
        || async {
            search::perform_search(&page, &query).await?;
            search::wait_for_results(&page).await?;
            search::extract_results(&page).await
        },
        MAX_RETRIES,
    )
    .await?;

    info!(
        "Search completed successfully with {} results",
        results.len()
    );
    Ok(SearchResults::new(query, results))
}

/// Perform web search (convenience function for standalone scripts)
///
/// For MCP tools, use `search_with_manager()` instead.
/// For standalone scripts, call `shutdown_standalone()` before exiting to clean up browser.
///
/// # Example
/// ```no_run
/// use kodegen_citescrape::web_search;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let results = web_search::search("rust programming").await?;
///     println!("Found {} results", results.results.len());
///     
///     // Clean up browser before exit
///     web_search::shutdown_standalone().await?;
///     Ok(())
/// }
/// ```
pub async fn search(query: impl Into<String>) -> Result<SearchResults> {
    let manager = STANDALONE_MANAGER.get_or_init(BrowserManager::new);
    search_with_manager(manager, query).await
}

/// Shutdown the standalone browser instance
///
/// Call this before exiting your program to properly clean up the browser
/// process and prevent zombie Chrome processes.
///
/// Safe to call even if no searches have been performed (no-op).
/// Safe to call multiple times (subsequent calls are no-ops).
///
/// # Example
/// ```no_run
/// use kodegen_citescrape::web_search;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     web_search::search("query").await?;
///     web_search::shutdown_standalone().await?;
///     Ok(())
/// }
/// ```
pub async fn shutdown_standalone() -> Result<()> {
    if let Some(manager) = STANDALONE_MANAGER.get() {
        manager.shutdown().await?;
    }
    Ok(())
}
