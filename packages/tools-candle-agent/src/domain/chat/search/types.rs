//! Search system type definitions and data structures
//!
//! This module contains all the core type definitions, enums, and data structures
//! used throughout the domain search system, following the single responsibility principle.

use std::collections::HashMap;

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};

use crate::domain::chat::message::CandleSearchChatMessage as SearchChatMessage;

/// Search query with advanced filtering options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search terms
    pub terms: Vec<String>,
    /// Boolean operator (AND, OR, NOT)
    pub operator: QueryOperator,
    /// Date range filter
    pub date_range: Option<DateRange>,
    /// User filter
    pub user_filter: Option<String>,
    /// Session filter
    pub session_filter: Option<String>,
    /// Tag filter
    pub tag_filter: Option<Vec<String>>,
    /// Content type filter
    pub content_type_filter: Option<String>,
    /// Fuzzy matching enabled
    pub fuzzy_matching: bool,
    /// Maximum results
    pub max_results: usize,
    /// Result offset for pagination
    pub offset: usize,
    /// Sort order
    pub sort_order: SortOrder,
}

/// Query operator enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryOperator {
    /// All terms must match
    And,
    /// Any term must match
    Or,
    /// Terms must not match
    Not,
    /// Exact phrase match
    Phrase,
    /// Proximity search
    Proximity {
        /// Distance value for proximity-based ranking
        distance: u32,
    },
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Start timestamp
    pub start: u64,
    /// End timestamp
    pub end: u64,
}

/// Sort order enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    /// Sort by relevance score (default)
    Relevance,
    /// Sort by date (newest first)
    DateDescending,
    /// Sort by date (oldest first)
    DateAscending,
    /// Sort by user alphabetically
    UserAscending,
    /// Sort by user reverse alphabetically
    UserDescending,
}

/// Search result with relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Message that matched
    pub message: SearchChatMessage,
    /// Relevance score (0.0-1.0)
    pub relevance_score: f32,
    /// Matching terms
    pub matching_terms: Vec<String>,
    /// Highlighted content
    pub highlighted_content: Option<String>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Context messages (before/after)
    pub context: Vec<SearchChatMessage>,
    /// Match positions in the content
    pub match_positions: Vec<MatchPosition>,
    /// Search metadata
    pub metadata: Option<SearchResultMetadata>,
}

impl Default for SearchResult {
    fn default() -> Self {
        Self {
            message: SearchChatMessage {
                message: crate::domain::chat::message::CandleMessage::default(),
                relevance_score: 0.0,
                highlights: Vec::new(),
            },
            relevance_score: 0.0,
            matching_terms: Vec::new(),
            highlighted_content: None,
            tags: Vec::new(),
            context: Vec::new(),
            match_positions: Vec::new(),
            metadata: None,
        }
    }
}

impl MessageChunk for SearchResult {
    fn bad_chunk(error: String) -> Self {
        Self {
            message: SearchChatMessage {
                message: crate::domain::chat::message::CandleMessage::bad_chunk(error),
                relevance_score: 0.0,
                highlights: Vec::new(),
            },
            relevance_score: 0.0,
            matching_terms: Vec::new(),
            highlighted_content: None,
            tags: Vec::new(),
            context: Vec::new(),
            match_positions: Vec::new(),
            metadata: None,
        }
    }

    fn error(&self) -> Option<&str> {
        self.message.message.error()
    }
}

/// Match position in content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPosition {
    /// Start position in characters
    pub start: usize,
    /// End position in characters
    pub end: usize,
    /// Matched term
    pub term: String,
}

/// Result type for search index operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexOperationResult {
    /// Operation success status
    pub success: bool,
    /// Document ID that was processed
    pub document_id: String,
    /// Number of terms indexed
    pub terms_indexed: usize,
    /// Operation duration in milliseconds
    pub duration_ms: f64,
    /// Optional error message
    pub error: Option<String>,
}

impl Default for IndexOperationResult {
    fn default() -> Self {
        Self {
            success: true,
            document_id: String::new(),
            terms_indexed: 0,
            duration_ms: 0.0,
            error: None,
        }
    }
}

impl MessageChunk for IndexOperationResult {
    fn bad_chunk(error: String) -> Self {
        Self {
            success: false,
            document_id: String::new(),
            terms_indexed: 0,
            duration_ms: 0.0,
            error: Some(error),
        }
    }

    fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Search result metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultMetadata {
    /// Query processing time
    pub query_time_ms: f64,
    /// Index version used
    pub index_version: u32,
    /// Total matches before filtering
    pub total_matches: usize,
}

/// Search statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchStatistics {
    /// Total messages indexed
    pub total_messages: usize,
    /// Total unique terms
    pub total_terms: usize,
    /// Total search queries
    pub total_queries: usize,
    /// Average query time in milliseconds
    pub average_query_time: f64,
    /// Index size in bytes
    pub index_size: usize,
    /// Last index update timestamp
    pub last_index_update: u64,
}

/// Term frequency and document frequency for TF-IDF calculation
#[derive(Debug, Clone)]
pub struct TermFrequency {
    /// Term frequency in document
    pub tf: f32,
    /// Document frequency (how many docs contain this term)
    pub df: u32,
    /// Total number of documents
    pub total_docs: u32,
}

impl TermFrequency {
    /// Calculate TF-IDF score
    #[must_use]
    pub fn calculate_tfidf(&self) -> f32 {
        let tf = self.tf;
        #[allow(clippy::cast_precision_loss)]
        let idf = ((self.total_docs as f32) / (self.df as f32)).ln();
        tf * idf
    }
}

/// Inverted index entry
#[derive(Debug, Clone)]
pub struct IndexEntry {
    /// Document ID (message ID)
    pub doc_id: String,
    /// Term frequency in document
    pub term_frequency: f32,
    /// Positions of term in document
    pub positions: Vec<usize>,
}

/// Search error types
#[derive(Debug, Clone)]
pub enum SearchError {
    /// Index operation failed
    IndexError { reason: String },
    /// Query parsing failed
    QueryError { reason: String },
    /// Search execution failed
    SearchError { reason: String },
    /// Export operation failed
    ExportError { reason: String },
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::IndexError { reason } => write!(f, "Index error: {reason}"),
            SearchError::QueryError { reason } => write!(f, "Query error: {reason}"),
            SearchError::SearchError { reason } => write!(f, "Search error: {reason}"),
            SearchError::ExportError { reason } => write!(f, "Export error: {reason}"),
        }
    }
}

impl std::error::Error for SearchError {}

/// Processed query with metadata
#[derive(Debug, Clone)]
pub struct ProcessedQuery {
    /// Original query string
    pub original: String,
    /// Processed terms
    pub terms: Vec<String>,
    /// Expanded terms from synonyms
    pub expanded_terms: Vec<String>,
    /// Query operator
    pub operator: QueryOperator,
    /// Processing metadata
    pub metadata: QueryMetadata,
}

/// Query metadata
#[derive(Debug, Clone)]
pub struct QueryMetadata {
    /// Processing timestamp
    pub processed_at: u64,
    /// Processing time in microseconds
    pub processing_time_us: u64,
    /// Expansion applied
    pub expansion_applied: bool,
    /// Normalization applied
    pub normalization_applied: bool,
}

/// Search options for query processing
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Enable query expansion
    pub enable_query_expansion: bool,
    /// Expansion dictionary
    pub expansion_dictionary: HashMap<String, Vec<String>>,
    /// Enable fuzzy matching
    pub enable_fuzzy_matching: bool,
    /// Maximum edit distance for fuzzy matching
    pub max_edit_distance: u8,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            enable_query_expansion: false,
            expansion_dictionary: HashMap::new(),
            enable_fuzzy_matching: false,
            max_edit_distance: 2,
        }
    }
}

/// Ranking algorithm types
#[derive(Debug, Clone)]
pub enum RankingAlgorithm {
    /// TF-IDF based ranking
    TfIdf,
    /// BM25 ranking algorithm
    Bm25,
    /// Custom scoring function
    Custom,
}

/// Export format enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// XML format
    Xml,
    /// Plain text format
    Text,
}

/// Export options
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Export format
    pub format: ExportFormat,
    /// Include metadata
    pub include_metadata: bool,
    /// Include context messages
    pub include_context: bool,
    /// Maximum results to export
    pub max_results: Option<usize>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            include_metadata: true,
            include_context: false,
            max_results: None,
        }
    }
}
