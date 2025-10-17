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

/// Pre-initialize all LazyLock statics to prevent blocking in tokio runtime
/// 
/// MUST be called before #[tokio::main] or tokio::runtime::Runtime::new().
/// This forces all LazyLock initialization to happen synchronously in the main thread
/// before the async runtime starts, preventing "Cannot block the current thread" panics.
/// 
/// # LazyLocks Initialized
/// 
/// - `crawl_engine::rate_limiter::DOMAIN_LIMITERS` - LRU cache for domain rate limiters
/// - `crawl_engine::rate_limiter::BASE_TIME` - Base timestamp for rate calculations
/// 
/// # Example
/// 
/// ```rust
/// citescrape::preinit_lazy_statics(); // Call BEFORE tokio runtime
/// 
/// tokio::runtime::Runtime::new().unwrap().block_on(async {
///     // Your async code here
/// });
/// ```
pub fn preinit_lazy_statics() {
    // Force initialization of rate limiter LazyLocks by accessing them
    // This must happen on the main thread before tokio runtime starts
    
    use crawl_engine::rate_limiter;
    
    // Touch DOMAIN_LIMITERS by checking tracked domain count (safe read operation)
    let _ = rate_limiter::get_tracked_domain_count();
    
    // Touch BASE_TIME by doing a dummy rate limit check (will touch BASE_TIME internally)
    let _ = rate_limiter::check_crawl_rate_limit("https://example.com", 1.0);
    
    // LazyLocks are now initialized - safe to start tokio runtime
}

pub use config::CrawlConfig;
pub use content_saver::{save_json_data, CacheMetadata};
pub use crawl_engine::{
    ChromiumoxideCrawler,
    CrawlError, CrawlProgress, CrawlResult, Crawler, CrawlQueue,
};
pub use page_extractor::schema::*;
pub use runtime::{
    spawn_async, AsyncTask, AsyncStream,
    AsyncJsonSave, BrowserAction, CrawlRequest,
};
pub use utils::{get_mirror_path, get_uri_from_path};

// Test-accessible modules
pub use crawl_engine::rate_limiter as crawl_rate_limiter;
pub use page_extractor::link_rewriter;

// MCP Tools and Managers
pub use mcp::{
    // Tools
    StartCrawlTool,
    GetCrawlResultsTool,
    SearchCrawlResultsTool,
    WebSearchTool,
    // Managers
    CrawlSessionManager,
    SearchEngineCache,
    ManifestManager,
    // Utilities
    url_to_output_dir,
    // Types
    ActiveCrawlSession,
    ConfigSummary,
    CrawlManifest,
    CrawlStatus,
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


pub fn crawl(
    config: CrawlConfig,
    on_result: impl FnOnce(Result<(), CrawlError>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let crawler = ChromiumoxideCrawler::new(config);
        let result = crawler.crawl().await;
        on_result(result);
        // AsyncTask returns () since the actual result is passed to callback
    })
}
