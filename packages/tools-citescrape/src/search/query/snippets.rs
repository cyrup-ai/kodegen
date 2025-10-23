//! Enhanced snippet generation with multi-field support

use anyhow::Result;
use tantivy::snippet::SnippetGenerator;
use tantivy::{TantivyDocument, schema::Value};

use crate::search::engine::SearchEngine;

/// Enhanced snippet generation with multi-field support
pub(crate) struct SnippetGenerators {
    title_generator: Option<SnippetGenerator>,
    content_generator: Option<SnippetGenerator>,
}

impl SnippetGenerators {
    pub(crate) fn create(
        searcher: &tantivy::Searcher,
        query: &dyn tantivy::query::Query,
        schema: &crate::search::schema::SearchSchema,
    ) -> Result<Self> {
        // Create title snippet generator
        let title_generator = SnippetGenerator::create(searcher, query, schema.title)
            .map(|mut generator| {
                generator.set_max_num_chars(100); // Shorter for titles
                generator
            })
            .ok();

        // Create content snippet generator
        let content_generator = SnippetGenerator::create(searcher, query, schema.plain_content)
            .map(|mut generator| {
                generator.set_max_num_chars(250); // Longer for content
                generator
            })
            .ok();

        Ok(SnippetGenerators {
            title_generator,
            content_generator,
        })
    }

    pub(crate) fn generate_snippet(&self, doc: &TantivyDocument, engine: &SearchEngine) -> String {
        let schema = engine.schema();

        // Try to generate snippet from content first
        if let Some(ref generator) = self.content_generator {
            let snippet = generator.snippet_from_doc(doc);
            let html = snippet.to_html();
            if !html.trim().is_empty() {
                return html;
            }
        }

        // Try to generate snippet from title
        if let Some(ref generator) = self.title_generator {
            let snippet = generator.snippet_from_doc(doc);
            let html = snippet.to_html();
            if !html.trim().is_empty() {
                return format!("<strong>{html}</strong>");
            }
        }

        // Fallback to stored snippet
        doc.get_first(schema.snippet)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }
}
