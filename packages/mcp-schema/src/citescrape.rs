//! Citescrape tool argument schemas

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// START CRAWL
// ============================================================================

fn default_max_depth() -> u8 {
    3
}

fn default_true() -> bool {
    true
}

fn default_crawl_rate() -> f64 {
    2.0
}

/// Arguments for `start_crawl` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartCrawlArgs {
    /// Target URL to crawl (required)
    pub url: String,

    /// Output directory for crawled content
    #[serde(default)]
    pub output_dir: Option<String>,

    /// Maximum crawl depth (default: 3)
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,

    /// Maximum number of pages to crawl (default: unbounded)
    #[serde(default)]
    pub limit: Option<usize>,

    /// Save markdown format (default: true)
    #[serde(default = "default_true")]
    pub save_markdown: bool,

    /// Save screenshots (default: false for speed)
    #[serde(default)]
    pub save_screenshots: bool,

    /// Enable search indexing (default: true)
    #[serde(default = "default_true")]
    pub enable_search: bool,

    /// Crawl rate in requests per second (default: 2.0)
    #[serde(default = "default_crawl_rate")]
    pub crawl_rate_rps: f64,

    /// Allow subdomain crawling (default: false)
    #[serde(default)]
    pub allow_subdomains: bool,

    /// Content types to generate
    #[serde(default)]
    pub content_types: Option<Vec<String>>,
}

/// Prompt arguments for `start_crawl` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartCrawlPromptArgs {}

// ============================================================================
// GET CRAWL RESULTS
// ============================================================================

fn default_true_get() -> bool {
    true
}

/// Arguments for `get_crawl_results` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCrawlResultsArgs {
    /// Crawl ID from `start_crawl` (for active crawls)
    #[serde(default)]
    pub crawl_id: Option<String>,

    /// Output directory (alternative to `crawl_id` for completed crawls)
    #[serde(default)]
    pub output_dir: Option<String>,

    /// Include progress details (default: true)
    #[serde(default = "default_true_get")]
    pub include_progress: bool,

    /// List all crawled files (default: true)
    #[serde(default = "default_true_get")]
    pub list_files: bool,

    /// Filter file listing by type
    #[serde(default)]
    pub file_types: Option<Vec<String>>,
}

/// Prompt arguments for `get_crawl_results` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCrawlResultsPromptArgs {}

// ============================================================================
// SEARCH CRAWL RESULTS
// ============================================================================

fn default_search_limit() -> usize {
    10
}

fn default_true_search() -> bool {
    true
}

/// Arguments for `search_crawl_results` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchCrawlResultsArgs {
    /// Search query string
    pub query: String,

    /// Crawl ID (optional, alternative to `output_dir`)
    #[serde(default)]
    pub crawl_id: Option<String>,

    /// Output directory to search in (optional)
    #[serde(default)]
    pub output_dir: Option<String>,

    /// Maximum results to return (default: 10)
    #[serde(default = "default_search_limit")]
    pub limit: usize,

    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: usize,

    /// Enable result highlighting (default: true)
    #[serde(default = "default_true_search")]
    pub highlight: bool,
}

/// Prompt arguments for `search_crawl_results` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchCrawlResultsPromptArgs {}

// ============================================================================
// WEB SEARCH
// ============================================================================

/// Arguments for `web_search` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchArgs {
    /// Search query string (required)
    pub query: String,
}

/// Prompt arguments for `web_search` tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchPromptArgs {}
