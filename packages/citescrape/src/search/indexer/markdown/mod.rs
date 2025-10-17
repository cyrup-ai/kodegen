//! Markdown processing utilities for indexing

mod title;
mod plaintext;
mod footnote;
mod snippet;
mod inline;
mod helpers;

use anyhow::Result;
use std::path::Path;
use chrono::Utc;
use imstr::ImString;
use super::super::types::ProcessedMarkdown;

/// Process markdown content with optimized allocations
#[inline]
pub(crate) fn process_markdown_content_optimized(
    markdown: &str,
    url: &ImString,
    file_path: &Path,
) -> Result<ProcessedMarkdown> {
    // Extract title efficiently
    let title = title::extract_title_from_markdown_optimized(markdown);
    
    // Convert to plain text with minimal allocations
    let plain_content = plaintext::markdown_to_plain_text_optimized(markdown);
    
    // Generate snippet
    let snippet = snippet::generate_snippet_optimized(plain_content.as_str(), 200);
    
    // Calculate metadata
    let file_size = markdown.len() as u64;
    let word_count = plain_content.as_str().split_whitespace().count() as u64;
    let crawl_date = Utc::now();
    let path = file_path.to_string_lossy().into_owned();
    
    Ok(ProcessedMarkdown {
        url: url.clone(),
        path: ImString::from(path),
        title,
        raw_markdown: ImString::from(markdown),
        plain_content,
        snippet,
        crawl_date,
        file_size,
        word_count,
    })
}
