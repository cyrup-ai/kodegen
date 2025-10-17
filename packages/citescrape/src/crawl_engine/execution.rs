//! Simple crawling execution with callback-based completion
//!
//! This module provides the simple crawl_impl API that executes a crawl
//! and calls back with the final result. Uses NoOpProgress internally
//! for zero-overhead execution.

use anyhow::Result;
use std::path::PathBuf;

use crate::config::CrawlConfig;
use crate::page_extractor::link_rewriter::LinkRewriter;
use crate::runtime::{spawn_async, AsyncTask};

use super::core::{crawl_pages, NoOpProgress};

/// Core crawling implementation that handles browser setup, page processing, and cleanup
/// 
/// This function contains the main crawling logic including:
/// - Browser initialization and configuration
/// - Recursive URL crawling with depth control
/// - Page data extraction and link discovery
/// - Comprehensive logging and error handling
/// - Resource cleanup
/// 
/// This is a thin wrapper around crawl_pages that uses NoOpProgress
/// for zero-overhead execution (all progress calls are inlined away).
/// 
/// # Arguments
/// * `config` - Crawl configuration
/// * `link_rewriter` - Link rewriting manager
/// * `chrome_data_dir` - Optional Chrome data directory
/// * `on_result` - Callback invoked with the final result
/// 
/// # Returns
/// AsyncTask handle for the spawned crawl operation
pub fn crawl_impl(
    config: CrawlConfig,
    link_rewriter: LinkRewriter,
    chrome_data_dir: Option<PathBuf>,
    on_result: impl FnOnce(Result<Option<PathBuf>>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        // Use NoOpProgress - event publishing handled directly by crawl_pages
        let progress = NoOpProgress;
        let event_bus = config.event_bus().cloned();
        
        let result = crawl_pages(config, link_rewriter, chrome_data_dir, progress, event_bus).await;

        // Invoke callback with result
        on_result(result);
    })
}
