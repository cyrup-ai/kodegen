//! Search query type detection and parsing

use anyhow::Result;
use tantivy::query::Query;

use super::query_builders::{
    build_boolean_query_sync, build_field_query_sync, build_fuzzy_query_sync,
    build_phrase_query_sync, build_text_query_sync,
};
use crate::search::engine::SearchEngine;

/// Search query types for different search patterns
#[derive(Debug, Clone)]
pub enum SearchQueryType {
    /// Simple text search
    Text(String),
    /// Phrase search with quotes
    Phrase(String),
    /// Boolean search with AND/OR/NOT
    Boolean(String),
    /// Field-specific search
    Field { field: String, query: String },
    /// Fuzzy search for typos
    Fuzzy { term: String, distance: u8 },
}

impl SearchQueryType {
    /// Parse a query string to determine its type
    #[must_use]
    pub fn parse(query: &str) -> Self {
        let query = query.trim();

        // Check for phrase query (quoted)
        if query.starts_with('"') && query.ends_with('"') && query.len() > 2 {
            return SearchQueryType::Phrase(query[1..query.len() - 1].to_string());
        }

        // Check for field-specific query
        if let Some(colon_pos) = query.find(':') {
            let field = query[..colon_pos].trim().to_string();
            let field_query = query[colon_pos + 1..].trim().to_string();
            return SearchQueryType::Field {
                field,
                query: field_query,
            };
        }

        // Check for fuzzy query (ends with ~ or ~N)
        if query.contains('~')
            && query.len() > 1
            && let Some(tilde_pos) = query.rfind('~')
        {
            let term = query[..tilde_pos].to_string();
            let distance_str = &query[tilde_pos + 1..];

            let distance = if distance_str.is_empty() {
                1 // Default distance
            } else {
                distance_str.parse::<u8>().unwrap_or(1).min(3) // Max distance of 3
            };

            return SearchQueryType::Fuzzy { term, distance };
        }

        // Check for boolean operators
        if query.contains(" AND ") || query.contains(" OR ") || query.contains(" NOT ") {
            return SearchQueryType::Boolean(query.to_string());
        }

        // Default to simple text search
        SearchQueryType::Text(query.to_string())
    }
}

/// Parse a search query string into a Tantivy Query (synchronous version)
pub(crate) fn parse_query_sync(engine: &SearchEngine, query_str: &str) -> Result<Box<dyn Query>> {
    let query_type = SearchQueryType::parse(query_str);

    match query_type {
        SearchQueryType::Text(text) => build_text_query_sync(engine, &text),
        SearchQueryType::Phrase(phrase) => build_phrase_query_sync(engine, &phrase),
        SearchQueryType::Boolean(boolean_str) => build_boolean_query_sync(engine, &boolean_str),
        SearchQueryType::Field { field, query } => build_field_query_sync(engine, &field, &query),
        SearchQueryType::Fuzzy { term, distance } => {
            build_fuzzy_query_sync(engine, &term, distance)
        }
    }
}
