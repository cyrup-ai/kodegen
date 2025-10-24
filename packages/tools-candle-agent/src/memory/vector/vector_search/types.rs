//! Core type definitions for vector search

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use surrealdb::Value;

use crate::memory::utils::error::Result;

/// Type alias for request info callback function
/// Callback receives: (result_id, similarity, confidence) -> bool (accept/reject)
pub type RequestInfoCallback = Arc<dyn Fn(&str, f32, f32) -> bool + Send + Sync>;

/// Type alias for deferred search result with confidence
/// Format: (id, vector, similarity, metadata, confidence)
pub(crate) type DeferredResult = (String, Vec<f32>, f32, Option<HashMap<String, Value>>, f32);

/// Type alias for final search result
/// Format: (id, vector, similarity, metadata)
pub(crate) type FinalResult = (String, Vec<f32>, f32, Option<HashMap<String, Value>>);

/// Type alias for keyword search function - SYNCHRONOUS OPERATIONS
///
/// This function type represents a synchronous keyword search operation.
/// For concurrent execution, wrap the function call in a thread.
pub type KeywordSearchFn = Arc<
    dyn Fn(&str, Option<super::options::SearchOptions>) -> Result<Vec<SearchResult>> + Send + Sync,
>;

/// Search result with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique identifier of the vector
    pub id: String,
    /// Vector data (included based on search options)
    pub vector: Vec<f32>,
    /// Similarity score (0.0 to 1.0, higher is better)
    pub similarity: f32,
    /// Optional metadata associated with the vector
    pub metadata: Option<HashMap<String, Value>>,
    /// Search ranking information (optional)
    pub rank: Option<usize>,
    /// Combined score from multiple search strategies (for hybrid search)
    pub combined_score: Option<f32>,
    /// Cognitive processor decision confidence (0.0 to 1.0)
    pub decision_confidence: Option<f32>,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(id: String, vector: Vec<f32>, similarity: f32) -> Self {
        Self {
            id,
            vector,
            similarity,
            metadata: None,
            rank: None,
            combined_score: None,
            decision_confidence: None,
        }
    }

    /// Create with metadata
    pub fn with_metadata(
        id: String,
        vector: Vec<f32>,
        similarity: f32,
        metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            id,
            vector,
            similarity,
            metadata: Some(metadata),
            rank: None,
            combined_score: None,
            decision_confidence: None,
        }
    }

    /// Set the ranking position
    #[must_use]
    pub fn with_rank(mut self, rank: usize) -> Self {
        self.rank = Some(rank);
        self
    }

    /// Set the combined score for hybrid search
    #[must_use]
    pub fn with_combined_score(mut self, score: f32) -> Self {
        self.combined_score = Some(score);
        self
    }

    /// Set the decision confidence from cognitive processor
    #[must_use]
    pub fn with_decision_confidence(mut self, confidence: f32) -> Self {
        self.decision_confidence = Some(confidence);
        self
    }

    /// Get the effective score for sorting (combined_score if available, otherwise similarity)
    pub fn effective_score(&self) -> f32 {
        self.combined_score.unwrap_or(self.similarity)
    }

    /// Memory usage estimation in bytes
    pub fn memory_usage(&self) -> usize {
        self.id.len() +
        self.vector.len() * std::mem::size_of::<f32>() +
        self.metadata.as_ref().map(|m| m.len() * 64).unwrap_or(0) + // Approximate metadata size
        std::mem::size_of::<Self>()
    }
}
