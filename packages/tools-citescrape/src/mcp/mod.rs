//! MCP (Model Context Protocol) Tools for Web Crawling
//!
//! This module provides three production-ready MCP tools that enable LLMs to autonomously
//! crawl websites, monitor progress, and search crawled documentation using Tantivy
//! full-text search.
//!
//! ## Tools
//!
//! ### `start_crawl`
//! Initiates a background web crawl with automatic search indexing. Returns immediately
//! with a `crawl_id` for status tracking.
//!
//! **Features:**
//! - Background processing (non-blocking)
//! - Automatic Tantivy search indexing
//! - Rate limiting (default 2 req/sec)
//! - Multiple output formats: Markdown, HTML, JSON
//! - Screenshot capture support
//!
//! ### `get_crawl_results`
//! Checks crawl status and retrieves progress information or completion summary.
//! Supports both active crawls (by `crawl_id`) and completed crawls (by `output_dir`).
//!
//! **Features:**
//! - Real-time progress tracking
//! - File listing for completed crawls
//! - Manifest-based historical queries
//!
//! ### `search_crawl_results`
//! Full-text search across crawled documentation with advanced query syntax.
//!
//! **Query Types:**
//! - Text: `layout components` (searches all fields)
//! - Phrase: `"exact phrase"` (exact match)
//! - Boolean: `layout AND (components OR widgets)`
//! - Field: `title:layout` (search specific field)
//! - Fuzzy: `layot~2` (allows 2 character differences)
//!
//! **Features:**
//! - Pagination support
//! - Result highlighting
//! - Relevance scoring
//!
//! ## Architecture
//!
//! The MCP layer uses direct tokio async/await patterns with three core managers:
//!
//! - **`CrawlSessionManager`**: Tracks active crawls in memory using `tokio::sync::Mutex`.
//!   Sessions have a 30-minute TTL and are automatically cleaned up.
//!
//! - **`SearchEngineCache`**: Caches `SearchEngine` instances per `output_dir` to avoid
//!   expensive re-initialization. Uses double-checked locking for thread-safe caching.
//!
//! - **`ManifestManager`**: Persists crawl metadata to manifest.json files for historical
//!   queries and status tracking after crawl completion.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use kodegen_citescrape::{
//!     StartCrawlTool, GetCrawlResultsTool, SearchCrawlResultsTool,
//!     CrawlSessionManager, SearchEngineCache
//! };
//! use kodegen_mcp_tool::Tool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create shared managers (once at startup)
//! let session_manager = CrawlSessionManager::new();
//! let engine_cache = SearchEngineCache::new();
//!
//! // Create tools
//! let start_crawl = StartCrawlTool::new(
//!     session_manager.clone(),
//!     engine_cache.clone(),
//! );
//! let get_results = GetCrawlResultsTool::new(session_manager.clone());
//! let search = SearchCrawlResultsTool::new(
//!     session_manager.clone(),
//!     engine_cache.clone(),
//! );
//!
//! // Tools implement kodegen_mcp_tool::Tool trait for MCP registration
//! println!("Tool name: {}", StartCrawlTool::name());
//! println!("Description: {}", StartCrawlTool::description());
//! # Ok(())
//! # }
//! ```
//!
//! ## Workflow
//!
//! 1. **Start Crawl**: Call `start_crawl` with target URL and options → returns `crawl_id`
//! 2. **Monitor Progress**: Poll `get_crawl_results` with `crawl_id` → returns status/progress
//! 3. **Search Results**: Call `search_crawl_results` with query → returns ranked results
//!
//! ## Output Directory Structure
//!
//! Crawled content is organized as:
//! ```text
//! docs/
//! └── ratatui.rs/
//!     ├── manifest.json          # Crawl metadata
//!     ├── .search_index/         # Tantivy search index
//!     │   ├── meta.json
//!     │   └── ...
//!     ├── index.md               # Crawled markdown files
//!     ├── tutorials/
//!     │   └── hello-world.md
//!     └── ...
//! ```
//!
//! ## Manager Lifecycle
//!
//! Managers are designed to be long-lived and shared:
//! - Create once at application startup
//! - Clone when passing to tool constructors
//! - Arc-based internally for efficient sharing
//!
//! ## Error Handling
//!
//! Tools return `Result<Value, McpError>`:
//! - Invalid URL: "Invalid URL 'xyz': ..."
//! - Missing index: "Search index not found at ..."
//! - Not found: "Crawl not found. Check `crawl_id` or `output_dir`."
//!
//! Handle errors appropriately in your MCP server implementation.

pub mod get_crawl_results;
pub mod manager;
pub mod search_crawl_results;
pub mod start_crawl;
pub mod types;
pub mod validation;
pub mod web_search;

// Re-export main types for convenience
pub use types::{ActiveCrawlSession, ConfigSummary, CrawlManifest, CrawlStatus};

// Re-export managers and utilities
pub use manager::{CrawlSessionManager, ManifestManager, SearchEngineCache, url_to_output_dir};
pub use validation::ErrorContext;

// Re-export tools
pub use get_crawl_results::GetCrawlResultsTool;
pub use search_crawl_results::SearchCrawlResultsTool;
pub use start_crawl::StartCrawlTool;
pub use web_search::WebSearchTool;
