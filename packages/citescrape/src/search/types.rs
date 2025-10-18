//! Common types used across the search module
//!
//! This module contains shared data structures and types that are used
//! by multiple components within the search system.

use chrono::{DateTime, Utc};
use imstr::ImString;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Individual search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub path: String,
    pub url: String,
    pub title: String,
    pub excerpt: String,
    pub score: f32,
}

/// Processed markdown document data
#[derive(Debug, Clone)]
pub struct ProcessedMarkdown {
    pub url: ImString,
    pub path: ImString,
    pub title: ImString,
    pub raw_markdown: ImString,
    pub plain_content: ImString,
    pub snippet: ImString,
    pub crawl_date: DateTime<Utc>,
    pub file_size: u64,
    pub word_count: u64,
}

/// Phase of batch indexing operation
#[derive(Debug, Clone, PartialEq)]
pub enum IndexingPhase {
    Discovering,
    Indexing,
    Optimizing,
    Complete,
    Cancelled,
}

/// Progress information for batch indexing
#[derive(Debug, Clone)]
pub struct IndexProgress {
    pub processed: usize,
    pub total: usize,
    pub failed: usize,
    pub current_file: ImString,
    pub phase: IndexingPhase,
    pub files_discovered: usize,
    pub discovery_complete: bool,
    pub errors: Vec<(ImString, ImString)>, // (file_path, error_message)
    pub started_at: Instant,
    pub estimated_completion: Option<DateTime<Utc>>,
}
