//! Search functionality using Tantivy for markdown content indexing and retrieval
//!
//! This module provides production-quality search capabilities for markdown content
//! crawled and stored by the citescrape system. It supports dual indexing of both
//! raw markdown and plain text for comprehensive search functionality.

pub mod engine;
pub mod errors;
pub mod incremental;
pub mod indexer;
pub mod query;
pub mod runtime_helpers;
pub mod schema;
pub mod types;

pub use engine::SearchEngine;
pub use errors::{RetryConfig, SearchError, SearchResult};
pub use incremental::{IncrementalIndexingService, IndexingSender, MessagePriority};
pub use indexer::MarkdownIndexer;
pub use query::{SearchQueryBuilder, SearchQueryType, SearchResults, search, search_with_options};
pub use runtime_helpers::{fallback_task, retry_task};
pub use schema::{SchemaError, SchemaPerformanceInfo, SearchSchema, SearchSchemaBuilder};
pub use types::{IndexProgress, ProcessedMarkdown};

use anyhow::Result;

/// Initialize the search system with the given configuration
pub async fn initialize_search(config: crate::config::CrawlConfig) -> Result<SearchEngine> {
    SearchEngine::create(&config).await
}
