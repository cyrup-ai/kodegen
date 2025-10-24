//! Data structures and constants for web search functionality

use serde::{Deserialize, Serialize};
use serde_json::Value;

// =============================================================================
// Constants
// =============================================================================

/// `DuckDuckGo` search URL base
pub const SEARCH_URL: &str = "https://duckduckgo.com";

/// CSS selector for individual search results
/// `DuckDuckGo` uses article elements with data-testid="result"
pub const SEARCH_RESULT_SELECTOR: &str = "article[data-testid='result']";

/// CSS selector for result titles (also contains the URL)
/// `DuckDuckGo` uses h2 > a structure for title links
pub const TITLE_SELECTOR: &str = "h2 > a";

/// CSS selector for result snippets/descriptions
/// `DuckDuckGo` uses div with data-result="snippet" attribute
pub const SNIPPET_SELECTOR: &str = "div[data-result='snippet']";

/// CSS selector for result links (same as title selector)
/// The title link contains the href attribute with the URL
pub const LINK_SELECTOR: &str = "h2 > a";

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
    /// Convert to `serde_json::Value` for MCP response
    #[must_use]
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
    /// Create new `SearchResults`
    #[must_use]
    pub fn new(query: String, results: Vec<SearchResult>) -> Self {
        Self { query, results }
    }

    /// Convert to `serde_json::Value` for MCP response
    #[must_use]
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "query": self.query,
            "result_count": self.results.len(),
            "results": self.results.iter().map(SearchResult::to_json).collect::<Vec<_>>()
        })
    }
}
