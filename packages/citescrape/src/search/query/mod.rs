//! Search query execution and result processing
//!
//! This module handles search query parsing, execution, and result formatting
//! with support for multiple query types and result highlighting.

use crate::search::engine::SearchEngine;

// Internal modules
mod builder;
mod results;
mod execution;
mod parsing;
mod query_builders;
mod snippets;

// Public exports
pub use builder::SearchQueryBuilder;
pub use results::SearchResults;
pub use parsing::SearchQueryType;

use anyhow::Result;
use crate::runtime::AsyncTask;
use crate::search::types::SearchResultItem;

/// Convenience function for simple search queries with logging
pub fn search(
    engine: &SearchEngine, 
    query: &str, 
    limit: Option<usize>
) -> AsyncTask<Result<Vec<SearchResultItem>>> {
    let query = query.to_string();
    let engine = engine.clone();
    let limit = limit.unwrap_or(10);
    
    crate::runtime::spawn_async(async move {
        let start = std::time::Instant::now();
        
        let result = SearchQueryBuilder::new(&query)
            .limit(limit)
            .execute(engine)
            .await??;
            
        let duration = start.elapsed();
        tracing::info!(
            query = %query,
            limit = limit,
            results_count = result.len(),
            duration_ms = duration.as_millis(),
            "Search completed successfully"
        );
        
        Ok(result)
    })
}

/// Advanced search function with full configuration options and logging
pub fn search_with_options(
    engine: &SearchEngine,
    query: &str,
    limit: Option<usize>,
    offset: Option<usize>, 
    highlight: Option<bool>,
) -> AsyncTask<Result<Vec<SearchResultItem>>> {
    let query = query.to_string();
    let engine = engine.clone();
    let limit = limit.unwrap_or(10);
    let offset = offset.unwrap_or(0);
    let highlight = highlight.unwrap_or(true);
    
    crate::runtime::spawn_async(async move {
        let start = std::time::Instant::now();
        
        let result = SearchQueryBuilder::new(&query)
            .limit(limit)
            .offset(offset)
            .highlight(highlight)
            .execute(engine)
            .await??;
            
        let duration = start.elapsed();
        tracing::info!(
            query = %query,
            limit = limit,
            offset = offset,
            highlight = highlight,
            results_count = result.len(),
            duration_ms = duration.as_millis(),
            "Advanced search completed successfully"
        );
        
        Ok(result)
    })
}
