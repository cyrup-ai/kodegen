//! Search functionality using Tantivy for markdown content indexing and retrieval
//!
//! This module provides production-quality search capabilities for markdown content
//! crawled and stored by the citescrape system. It supports dual indexing of both
//! raw markdown and plain text for comprehensive search functionality.

pub mod schema;
pub mod engine;
pub mod indexer;
pub mod query;
pub mod types;
pub mod incremental;
pub mod errors;
pub mod runtime_helpers;

pub use engine::SearchEngine;
pub use indexer::MarkdownIndexer;
pub use query::{SearchQueryBuilder, SearchResults, SearchQueryType, search, search_with_options};
pub use schema::{SearchSchema, SearchSchemaBuilder, SchemaError, SchemaPerformanceInfo};
pub use types::{ProcessedMarkdown, IndexProgress};
pub use incremental::{IncrementalIndexingService, IndexingSender, MessagePriority};
pub use errors::{SearchError, SearchResult, RetryConfig};
pub use runtime_helpers::{retry_task, fallback_task, CancellableTask, RateLimitedTask, BatchedTask};

use anyhow::Result;
use crate::runtime::AsyncTask;

/// Initialize the search system with the given configuration
pub fn initialize_search(config: crate::config::CrawlConfig) -> AsyncTask<Result<SearchEngine>> {
    crate::runtime::spawn_async(async move {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = SearchEngine::create_async(&config, move |result| {
            let _ = tx.send(result);
        });
        let _guard = crate::runtime::TaskGuard::new(task, "SearchEngine::create_async");
        match rx.await.map_err(|_| anyhow::anyhow!("Failed to initialize search engine")) {
            Ok(result) => result,
            Err(e) => Err(anyhow::anyhow!(e))
        }
    })
}