//! Data structures and constants for web search functionality

use serde::{Deserialize, Serialize};
use serde_json::Value;

// =============================================================================
// Constants
// =============================================================================

/// Google search homepage URL
pub const SEARCH_URL: &str = "https://www.google.com";

/// CSS selector for the search input box
pub const SEARCH_BOX_SELECTOR: &str = "textarea[name='q']";

/// CSS selector for the search button
pub const SEARCH_BUTTON_SELECTOR: &str = "input[name='btnK'], button[name='btnK']";

/// CSS selector for individual search results
pub const SEARCH_RESULT_SELECTOR: &str = "div.g";

/// CSS selector for result titles
pub const TITLE_SELECTOR: &str = "h3";

/// CSS selector for result snippets
pub const SNIPPET_SELECTOR: &str = "div.VwiC3b";

/// CSS selector for result links
pub const LINK_SELECTOR: &str = "div.yuRUbf > a";

/// Maximum time to wait for search results (seconds)
pub const SEARCH_RESULTS_WAIT_TIMEOUT: u64 = 10;

/// Maximum number of retry attempts
pub const MAX_RETRIES: u32 = 3;

/// Maximum number of results to extract
pub const MAX_RESULTS: usize = 10;

// =============================================================================
// Data Structures
// =============================================================================

/// A single search result with rank, title, URL, and snippet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Result ranking (1-indexed)
    pub rank: usize,

    /// Page title
    pub title: String,

    /// Page URL
    pub url: String,

    /// Description snippet from search results
    pub snippet: String,
}

impl SearchResult {
    /// Convert to serde_json::Value for MCP response
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "rank": self.rank,
            "title": self.title,
            "url": self.url,
            "snippet": self.snippet
        })
    }
}

/// Collection of search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Search query that produced these results
    pub query: String,

    /// List of search results
    pub results: Vec<SearchResult>,
}

impl SearchResults {
    /// Create new SearchResults
    pub fn new(query: String, results: Vec<SearchResult>) -> Self {
        Self { query, results }
    }

    /// Convert to serde_json::Value for MCP response
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "query": self.query,
            "result_count": self.results.len(),
            "results": self.results.iter().map(|r| r.to_json()).collect::<Vec<_>>()
        })
    }
}
