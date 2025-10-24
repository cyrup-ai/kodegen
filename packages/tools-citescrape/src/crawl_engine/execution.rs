//! Simple async crawling execution
//!
//! This module provides the simple `crawl_impl` async API that executes a crawl
//! and returns the final result. Uses `NoOpProgress` internally
//! for zero-overhead execution.

use anyhow::Result;
use std::path::PathBuf;

use crate::config::CrawlConfig;
use crate::page_extractor::link_rewriter::LinkRewriter;

use super::core::{NoOpProgress, crawl_pages};

/// Core crawling implementation that handles browser setup, page processing, and cleanup
///
/// This function contains the main crawling logic including:
/// - Browser initialization and configuration
/// - Recursive URL crawling with depth control
/// - Page data extraction and link discovery
/// - Comprehensive logging and error handling
/// - Resource cleanup
///
/// This is a thin wrapper around `crawl_pages` that uses `NoOpProgress`
/// for zero-overhead execution (all progress calls are inlined away).
///
/// # Arguments
/// * `config` - Crawl configuration
/// * `link_rewriter` - Link rewriting manager
/// * `chrome_data_dir` - Optional Chrome data directory
///
/// # Returns
/// `Result<Option<PathBuf>>` - Returns the chrome data directory path on success
pub async fn crawl_impl(
    config: CrawlConfig,
    link_rewriter: LinkRewriter,
    chrome_data_dir: Option<PathBuf>,
) -> Result<Option<PathBuf>> {
    // Use NoOpProgress - event publishing handled directly by crawl_pages
    let progress = NoOpProgress;
    let event_bus = config.event_bus().cloned();

    crawl_pages(config, link_rewriter, chrome_data_dir, progress, event_bus).await
}
