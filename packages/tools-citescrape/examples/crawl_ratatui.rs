//! Full site crawl example for ratatui.rs documentation
//!
//! This example demonstrates:
//! - Full site crawling with all output formats (PNG, HTML, MD, JSON)
//! - Automatic Tantivy search indexing during crawl
//! - Search verification using existing `SearchEngine` infrastructure

use kodegen_tools_citescrape::search::{
    IncrementalIndexingService, SearchEngine, SearchQueryBuilder,
};
use kodegen_tools_citescrape::{CrawlConfig, crawl};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize detailed logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("🚀 Starting FULL SITE crawl of ratatui.rs documentation");
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Configure crawl with ALL features enabled
    let output_dir = PathBuf::from("docs/ratatui.rs");
    let search_index_dir = output_dir.join(".search_index");

    let config = CrawlConfig::builder()
        .storage_dir(output_dir.clone())
        .start_url("https://ratatui.rs/")
        .limit(None) // ✅ Crawl ALL pages (no limit)
        .max_depth(10) // ✅ Deep crawl (all levels)
        .save_markdown(true) // ✅ Save markdown
        // JSON is always saved by default (no setter method exists)
        .save_raw_html(true) // ✅ Save HTML
        .save_screenshots(true) // ✅ Save PNG screenshots
        .search_index_dir(Some(search_index_dir.clone())) // ✅ Enable Tantivy search (automatic indexing)
        .crawl_rate_rps(2.0) // Polite: 2 pages per second
        .allow_subdomains(false) // Stay on ratatui.rs only
        .build()
        .expect("Failed to build config");

    log::info!("📋 Crawl configuration:");
    log::info!("  URL: {}", config.start_url());
    log::info!("  Output: {}", config.storage_dir().display());
    log::info!("  Max depth: {}", config.max_depth());
    log::info!(
        "  Page limit: {:?}",
        config
            .limit()
            .map_or("unlimited".to_string(), |n| n.to_string())
    );
    log::info!("  Formats: markdown ✓, json ✓, html ✓, screenshots ✓");
    log::info!(
        "  Search index: {} (automatic indexing enabled)",
        search_index_dir.display()
    );
    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Initialize search engine and incremental indexing (exactly like MCP path)
    log::info!("🔧 Initializing search engine and incremental indexing...");
    let engine = SearchEngine::create(&config).await?;
    let indexing_sender = IncrementalIndexingService::start(engine).await?;

    // Attach indexing sender to config (exactly like MCP does in start_crawl.rs)
    let config = config.with_indexing_sender(Arc::new(indexing_sender));
    log::info!("✅ Incremental indexing service started");

    // Execute crawl (search indexing happens automatically during crawl)
    log::info!("🕷️  Starting crawl with automatic search indexing...");
    match crawl(config.clone()).await {
        Ok(()) => {
            log::info!("✅ Crawl completed successfully!");
            log::info!("📂 Results saved to: {}", output_dir.display());
        }
        Err(e) => {
            log::error!("❌ Crawl failed: {e:#}");
            return Err(e.into());
        }
    }

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Verify search functionality using existing SearchEngine infrastructure
    log::info!("🔎 Verifying Tantivy search using SearchEngine...");
    test_search(&config).await?;

    log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    log::info!(
        "✅ ALL TESTS PASSED! Full site crawl with automatic search indexing completed successfully."
    );

    Ok(())
}

/// Test Tantivy search using the existing `SearchEngine` infrastructure
async fn test_search(config: &CrawlConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Use existing SearchEngine::create() instead of manually opening Tantivy index
    let engine = SearchEngine::create(config).await?;

    // Get index statistics using existing get_stats() method
    let stats = engine.get_stats().await?;

    log::info!("📈 Search index statistics:");
    log::info!("  Documents indexed: {}", stats.num_documents);
    log::info!("  Index segments: {}", stats.num_segments);
    if let Some(size) = stats.index_size_bytes {
        log::info!("  Index size: ~{:.2} MB", size as f64 / 1_048_576.0);
    }
    if let Some(last_commit) = stats.last_commit {
        log::info!("  Last commit: {}", last_commit.format("%Y-%m-%d %H:%M:%S"));
    }

    // Verify we have indexed content
    if stats.num_documents == 0 {
        return Err("No documents indexed! Search indexing may have failed.".into());
    }

    log::info!("");
    log::info!("📝 Running test queries using SearchQueryBuilder:");
    log::info!("");

    // Test queries using existing SearchQueryBuilder API
    let test_queries = vec![
        ("ratatui", "Basic search for 'ratatui'"),
        ("installation", "Search for installation docs"),
        ("tutorial", "Search for tutorials"),
        ("widget", "Search for widget information"),
    ];

    for (query_str, description) in test_queries {
        log::info!("  Query: \"{query_str}\" ({description})");

        // Use SearchQueryBuilder for clean, fluent API
        let results = SearchQueryBuilder::new(query_str)
            .limit(5)
            .highlight(true)
            .execute(engine.clone())
            .await?;

        if results.is_empty() {
            log::warn!("    ⚠️  No results found");
        } else {
            log::info!("    ✅ Found {} results:", results.len());
            for (i, item) in results.iter().enumerate() {
                log::info!("      {}. {} (score: {:.2})", i + 1, item.title, item.score);
                log::info!("         URL: {}", item.url);
            }
        }
        log::info!("");
    }

    log::info!("✅ Search functionality verified using existing SearchEngine infrastructure");
    Ok(())
}
