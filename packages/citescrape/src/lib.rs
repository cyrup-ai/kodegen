pub mod config;
pub mod content_saver;
pub mod crawl_engine;
pub mod crawl_events;
pub mod inline_css;
pub mod kromekover;
pub mod mcp;
pub mod page_extractor;
pub mod runtime;
pub mod search;
pub mod utils;
pub mod web_search;

pub use config::CrawlConfig;
pub use content_saver::{CacheMetadata, save_json_data};
pub use crawl_engine::{
    ChromiumoxideCrawler, CrawlError, CrawlProgress, CrawlQueue, CrawlResult, Crawler,
};
pub use page_extractor::schema::*;
pub use runtime::{
    AsyncJsonSave, AsyncStream, BrowserAction, CrawlRequest,
};
pub use utils::{get_mirror_path, get_uri_from_path};

// Test-accessible modules
pub use crawl_engine::rate_limiter as crawl_rate_limiter;
pub use page_extractor::link_rewriter;

// MCP Tools and Managers
pub use mcp::{
    // Types
    ActiveCrawlSession,
    ConfigSummary,
    CrawlManifest,
    // Managers
    CrawlSessionManager,
    CrawlStatus,
    GetCrawlResultsTool,
    ManifestManager,
    SearchCrawlResultsTool,
    SearchEngineCache,
    // Tools
    StartCrawlTool,
    WebSearchTool,
    // Utilities
    url_to_output_dir,
};

/// Macro for handling streaming data chunks with safe unwrapping
#[macro_export]
macro_rules! on_chunk {
    ($closure:expr) => {
        move |chunk| match chunk {
            Ok(data) => $closure(data),
            Err(e) => {
                eprintln!("Chunk error: {:?}", e);
            }
        }
    };
}

/// Macro for handling errors with safe unwrapping
#[macro_export]
macro_rules! on_error {
    ($closure:expr) => {
        move |error| match error {
            Some(e) => $closure(e),
            None => {
                eprintln!("Unknown error occurred");
            }
        }
    };
}

pub async fn crawl(config: CrawlConfig) -> Result<(), CrawlError> {
    let crawler = ChromiumoxideCrawler::new(config);
    crawler.crawl().await
}
