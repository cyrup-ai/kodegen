//! Search results container and document conversion

use anyhow::Result;
use tantivy::{TantivyDocument, schema::Value};

use super::snippets::SnippetGenerators;
use crate::search::engine::SearchEngine;
use crate::search::types::SearchResultItem;

/// Search results container
#[derive(Debug, Clone)]
pub struct SearchResults {
    pub results: Vec<SearchResultItem>,
    pub total_count: usize,
    pub query: String,
    pub offset: usize,
    pub limit: usize,
}

impl SearchResults {
    /// Check if there are more results available
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.offset + self.results.len() < self.total_count
    }

    /// Get the next page offset
    #[must_use]
    pub fn next_offset(&self) -> Option<usize> {
        if self.has_more() {
            Some(self.offset + self.results.len())
        } else {
            None
        }
    }
}

/// Convert a Tantivy document to a `SearchResultItem` with enhanced snippet generation
pub(crate) fn convert_to_search_result(
    doc: &TantivyDocument,
    engine: &SearchEngine,
    score: f32,
    generators: Option<&SnippetGenerators>,
) -> Result<SearchResultItem> {
    let schema = engine.schema();

    // Extract fields from document with proper error handling
    let url = doc
        .get_first(schema.url)
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown URL")
        .to_string();

    let title = doc
        .get_first(schema.title)
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let path = doc
        .get_first(schema.path)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Generate enhanced snippet with generators or fallback
    let excerpt = if let Some(gens) = generators {
        // Use SnippetGenerators for context-aware highlighting
        gens.generate_snippet(doc, engine)
    } else {
        // Fallback to stored snippet
        doc.get_first(schema.snippet)
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| {
                // Final fallback: truncate content if available
                doc.get_first(schema.plain_content)
                    .and_then(|v| v.as_str())
                    .map(|content| {
                        if content.chars().count() > 200 {
                            let truncated = content.chars().take(200).collect::<String>();
                            format!("{truncated}...")
                        } else {
                            content.to_string()
                        }
                    })
                    .unwrap_or_else(|| "No content available".to_string())
            })
    };

    Ok(SearchResultItem {
        path,
        url,
        title,
        excerpt,
        score,
    })
}
