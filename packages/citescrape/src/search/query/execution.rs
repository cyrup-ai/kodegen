//! Search query execution logic

use anyhow::Result;
use tantivy::collector::{TopDocs, Count};

use crate::runtime::{AsyncTask, spawn_async};
use crate::search::engine::SearchEngine;
use crate::search::errors::{SearchError, SearchResult};
use crate::search::runtime_helpers::fallback_task;
use super::results::{SearchResults, convert_to_search_result};
use super::snippets::SnippetGenerators;
use super::parsing::parse_query_sync;

/// Execute a search query against the index with fallback behavior
pub(crate) fn execute_search_query(
    engine: &SearchEngine,
    query_str: &str,
    limit: usize,
    offset: usize,
    highlight: bool,
) -> AsyncTask<Result<SearchResults>> {
    let engine = engine.clone();
    let query_str = query_str.to_string();
    
    // Clone values for closures
    let engine_primary = engine.clone();
    let query_primary = query_str.clone();
    let engine_fallback = engine.clone();
    let query_fallback = query_str.clone();
    
    // Use fallback_task for primary and fallback search
    let task = fallback_task(
        move || {
            execute_search_with_features(engine_primary, query_primary, limit, offset, highlight)
        },
        move || {
            tracing::warn!("Attempting fallback search with reduced features");
            execute_search_with_features(engine_fallback, query_fallback, limit, offset, false)
        }
    );
    
    // Convert SearchResult<SearchResults> to Result<SearchResults>
    spawn_async(async move {
        task.into_anyhow().await?
            .map_err(|e| anyhow::anyhow!("{}", e))
    })
}

/// Internal search execution with configurable features
fn execute_search_with_features(
    engine: SearchEngine,
    query_str: String,
    limit: usize,
    offset: usize,
    highlight: bool,
) -> AsyncTask<SearchResult<SearchResults>> {
    spawn_async(async move {
        let reader = engine.reader();
        let searcher = reader.searcher();

        // Parse the query
        let query = parse_query_sync(&engine, &query_str)
            .map_err(|e| SearchError::QueryParsing(format!("Failed to parse query: {}", e)))?;

        // Create snippet generators if highlighting is enabled
        let generators = if highlight {
            SnippetGenerators::create(&searcher, &*query, engine.schema())
                .ok()
        } else {
            None
        };

        // Get total count
        let total_count = searcher.search(&*query, &Count)
            .map_err(|e| SearchError::SearchExecution(format!("Failed to count search results: {}", e)))?;

        // Execute search with pagination
        let top_docs = searcher.search(&*query, &TopDocs::with_limit(limit).and_offset(offset))
            .map_err(|e| SearchError::SearchExecution(format!("Failed to execute search query: {}", e)))?;

        // Convert results to SearchResult objects
        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)
                .map_err(|e| SearchError::DocumentNotFound(format!("Failed to retrieve document: {}", e)))?;

            let search_result = convert_to_search_result(
                &doc,
                &engine,
                score,
                generators.as_ref(),
            ).map_err(|e| SearchError::Other(format!("Failed to convert search result: {}", e)))?;

            results.push(search_result);
        }

        Ok(SearchResults {
            results,
            total_count,
            query: query_str,
            offset,
            limit,
        })
    })
}
