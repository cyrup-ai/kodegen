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
//! # Example
//! ```no_run
//! use kodegen_citescrape::web_search;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let results = web_search::search("rust programming").await?;
//!     println!("Found {} results", results.results.len());
//!     Ok(())
//! }
//! ```

mod browser;
mod search;
mod types;

// Re-export public types
pub use types::{MAX_RESULTS, MAX_RETRIES, SearchResult, SearchResults};

use anyhow::Result;
use tracing::info;

/// Perform a web search and return structured results
///
/// This is the main entry point for web search functionality. It:
/// 1. Gets or launches the shared browser (instant after first call)
/// 2. Creates a fresh blank page for this search
/// 3. Performs the search query (with kromekover stealth features)
/// 4. Extracts results with retry logic
/// 5. Keeps browser alive for subsequent searches
///
/// # Performance
/// - First search: ~5.5s (includes browser launch)
/// - Subsequent searches: ~2-3s (60% faster, no launch overhead)
///
/// # Arguments
/// * `query` - Search query string
///
/// # Returns
/// SearchResults containing the query and extracted results
///
/// # Errors
/// Returns error if:
/// - Browser launch fails (first call only)
/// - Page creation fails
/// - Search execution fails after retries
/// - Result extraction fails
///
/// # Based on
/// - packages/citescrape/src/google_search.rs:27-81 (orchestration pattern)
/// - packages/citescrape/src/crawl_engine/core.rs:231-259 (stealth pattern)
pub async fn search(query: impl Into<String>) -> Result<SearchResults> {
    let query = query.into();
    info!("Starting web search for query: {}", query);

    // Get shared browser (instant if already launched, 2-3s on first call)
    let browser_wrapper = browser::get_or_launch_browser().await?;

    // Create fresh BLANK page for kromekover stealth injection
    // Each search gets its own page for isolation
    let page = browser::create_blank_page(&browser_wrapper).await?;

    // Perform search with retry logic
    let results = search::retry_with_backoff(
        || async {
            // Execute search
            search::perform_search(&page, &query).await?;

            // Wait for results to load
            search::wait_for_results(&page).await?;

            // Extract results
            search::extract_results(&page).await
        },
        MAX_RETRIES,
    )
    .await?;

    // ✓ DON'T close browser - keep it alive for next search

    info!(
        "Search completed successfully with {} results",
        results.len()
    );

    Ok(SearchResults::new(query, results))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires browser installation
    async fn test_search_basic() {
        let results = search("rust programming").await.unwrap();
        assert!(!results.results.is_empty());
        assert_eq!(results.query, "rust programming");
    }
}
