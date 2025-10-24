//! Query builders for different search types

use anyhow::Result;
use tantivy::{
    Term,
    query::{BooleanQuery, FuzzyTermQuery, Occur, PhraseQuery, Query, TermQuery},
};

use crate::search::engine::SearchEngine;

// ====================
// SYNCHRONOUS VERSIONS
// ====================

/// Build a multi-field text query searching both title and content (synchronous version)
pub(crate) fn build_text_query_sync(engine: &SearchEngine, text: &str) -> Result<Box<dyn Query>> {
    let schema = engine.schema();

    // Collect all sub-queries
    let mut subqueries = Vec::new();

    // Add title field search with higher boost
    for word in text.split_whitespace() {
        let term = Term::from_field_text(schema.title, word);
        let term_query = TermQuery::new(
            term,
            tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
        );
        subqueries.push((Occur::Should, Box::new(term_query) as Box<dyn Query>));
    }

    // Add content field search
    for word in text.split_whitespace() {
        let term = Term::from_field_text(schema.plain_content, word);
        let term_query = TermQuery::new(
            term,
            tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
        );
        subqueries.push((Occur::Should, Box::new(term_query) as Box<dyn Query>));
    }

    // Also search raw markdown for exact matches
    for word in text.split_whitespace() {
        let term = Term::from_field_text(schema.raw_markdown, word);
        let term_query = TermQuery::new(term, tantivy::schema::IndexRecordOption::Basic);
        subqueries.push((Occur::Should, Box::new(term_query) as Box<dyn Query>));
    }

    let boolean_query = BooleanQuery::new(subqueries);
    Ok(Box::new(boolean_query) as Box<dyn Query>)
}

/// Build a phrase query for exact phrase matching (synchronous version)
pub(crate) fn build_phrase_query_sync(
    engine: &SearchEngine,
    phrase: &str,
) -> Result<Box<dyn Query>> {
    let schema = engine.schema();

    // Parse phrase into terms
    let terms: Vec<String> = phrase
        .split_whitespace()
        .map(std::string::ToString::to_string)
        .collect();

    if terms.is_empty() {
        return Err(anyhow::anyhow!("Empty phrase query"));
    }

    // Collect sub-queries for phrase searches across multiple fields
    let mut subqueries = Vec::new();

    // Title phrase query (higher relevance)
    if terms.len() == 1 {
        let term = Term::from_field_text(schema.title, &terms[0]);
        let term_query = TermQuery::new(
            term,
            tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
        );
        subqueries.push((Occur::Should, Box::new(term_query) as Box<dyn Query>));
    } else {
        let title_terms: Vec<Term> = terms
            .iter()
            .map(|term| Term::from_field_text(schema.title, term))
            .collect();
        let phrase_query = PhraseQuery::new(title_terms);
        subqueries.push((Occur::Should, Box::new(phrase_query) as Box<dyn Query>));
    }

    // Content phrase query
    if terms.len() == 1 {
        let term = Term::from_field_text(schema.plain_content, &terms[0]);
        let term_query = TermQuery::new(
            term,
            tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
        );
        subqueries.push((Occur::Should, Box::new(term_query) as Box<dyn Query>));
    } else {
        let content_terms: Vec<Term> = terms
            .iter()
            .map(|term| Term::from_field_text(schema.plain_content, term))
            .collect();
        let phrase_query = PhraseQuery::new(content_terms);
        subqueries.push((Occur::Should, Box::new(phrase_query) as Box<dyn Query>));
    }

    let boolean_query = BooleanQuery::new(subqueries);
    Ok(Box::new(boolean_query) as Box<dyn Query>)
}

/// Build a boolean query using tantivy's query parser with field mapping (synchronous version)
pub(crate) fn build_boolean_query_sync(
    engine: &SearchEngine,
    query_str: &str,
) -> Result<Box<dyn Query>> {
    let query_parser = engine.query_parser();

    // Try to parse as a standard boolean query first
    match query_parser.parse_query(query_str) {
        Ok(query) => Ok(query),
        Err(parse_error) => {
            // Log warning so users know fallback occurred
            tracing::warn!(
                query = %query_str,
                error = %parse_error,
                "Boolean query parsing failed, falling back to text search"
            );

            // Fallback to text query if boolean parsing fails
            build_text_query_sync(engine, query_str)
        }
    }
}

/// Build a field-specific query (synchronous version)
pub(crate) fn build_field_query_sync(
    engine: &SearchEngine,
    field_name: &str,
    query: &str,
) -> Result<Box<dyn Query>> {
    let schema = engine.schema();

    // Map field names to schema fields
    let field = match field_name.to_lowercase().as_str() {
        "title" => schema.title,
        "content" | "text" => schema.plain_content,
        "markdown" | "raw" => schema.raw_markdown,
        "url" => schema.url,
        "path" => schema.path,
        _ => {
            return Err(anyhow::anyhow!("Unknown field: {field_name}"));
        }
    };

    // Check for fuzzy operator in query
    if let Some(tilde_pos) = query.rfind('~') {
        let term = &query[..tilde_pos];
        let distance_str = &query[tilde_pos + 1..];
        let distance = if distance_str.is_empty() {
            1
        } else {
            distance_str.parse::<u8>().unwrap_or(1).min(3)
        };

        let term_obj = Term::from_field_text(field, term);
        let fuzzy_query = FuzzyTermQuery::new(term_obj, distance, true);
        return Ok(Box::new(fuzzy_query) as Box<dyn Query>);
    }

    // Handle phrase vs single term
    if query.contains(' ') {
        // Multi-word query - treat as phrase
        let terms: Vec<Term> = query
            .split_whitespace()
            .map(|term| Term::from_field_text(field, term))
            .collect();

        if terms.len() == 1 {
            let term_query = TermQuery::new(
                terms[0].clone(),
                tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
            );
            Ok(Box::new(term_query) as Box<dyn Query>)
        } else {
            let phrase_query = PhraseQuery::new(terms);
            Ok(Box::new(phrase_query) as Box<dyn Query>)
        }
    } else {
        // Single term query
        let term = Term::from_field_text(field, query);
        let term_query = TermQuery::new(
            term,
            tantivy::schema::IndexRecordOption::WithFreqsAndPositions,
        );
        Ok(Box::new(term_query) as Box<dyn Query>)
    }
}

/// Build a fuzzy query for handling typos (synchronous version)
pub(crate) fn build_fuzzy_query_sync(
    engine: &SearchEngine,
    term_str: &str,
    distance: u8,
) -> Result<Box<dyn Query>> {
    let schema = engine.schema();

    // Create fuzzy queries for multiple fields
    let mut subqueries = Vec::new();

    // Title fuzzy search
    let title_term = Term::from_field_text(schema.title, term_str);
    let title_fuzzy = FuzzyTermQuery::new(title_term, distance, true);
    subqueries.push((Occur::Should, Box::new(title_fuzzy) as Box<dyn Query>));

    // Content fuzzy search
    let content_term = Term::from_field_text(schema.plain_content, term_str);
    let content_fuzzy = FuzzyTermQuery::new(content_term, distance, true);
    subqueries.push((Occur::Should, Box::new(content_fuzzy) as Box<dyn Query>));

    let boolean_query = BooleanQuery::new(subqueries);
    Ok(Box::new(boolean_query) as Box<dyn Query>)
}
