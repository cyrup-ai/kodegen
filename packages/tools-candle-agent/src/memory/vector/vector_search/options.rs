//! Search configuration and options

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use surrealdb::Value;

use super::types::RequestInfoCallback;
use crate::memory::utils::error::Result;

/// Search options for fine-tuning search behavior
#[derive(Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Minimum similarity threshold (0.0 to 1.0)
    pub min_similarity: Option<f32>,
    /// Metadata filters to apply (exact match)
    pub filters: Option<HashMap<String, Value>>,
    /// Whether to include vectors in results (affects memory usage)
    pub include_vectors: Option<bool>,
    /// Whether to include metadata in results
    pub include_metadata: Option<bool>,
    /// Whether to include ranking information
    pub include_rank: Option<bool>,
    /// Maximum number of results to consider before filtering (for performance)
    pub candidate_limit: Option<usize>,
    /// Whether to enable SIMD optimization (default: true)
    pub enable_simd: Option<bool>,
    /// Optional callback for RequestInfo outcomes requiring user interaction
    /// Callback receives: (result_id, similarity, confidence) -> bool (accept/reject)
    #[serde(skip)]
    pub request_info_callback: Option<RequestInfoCallback>,
}

impl std::fmt::Debug for SearchOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchOptions")
            .field("limit", &self.limit)
            .field("min_similarity", &self.min_similarity)
            .field("filters", &self.filters)
            .field("include_vectors", &self.include_vectors)
            .field("include_metadata", &self.include_metadata)
            .field("include_rank", &self.include_rank)
            .field("candidate_limit", &self.candidate_limit)
            .field("enable_simd", &self.enable_simd)
            .field(
                "request_info_callback",
                &self.request_info_callback.as_ref().map(|_| "<callback>"),
            )
            .finish()
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: Some(10),
            min_similarity: Some(0.7),
            filters: None,
            include_vectors: Some(false),
            include_metadata: Some(true),
            include_rank: Some(false),
            candidate_limit: Some(1000),
            enable_simd: Some(true),
            request_info_callback: None,
        }
    }
}

impl SearchOptions {
    /// Create options optimized for performance (minimal data returned)
    pub fn fast() -> Self {
        Self {
            limit: Some(10),
            min_similarity: Some(0.8),
            filters: None,
            include_vectors: Some(false),
            include_metadata: Some(false),
            include_rank: Some(false),
            candidate_limit: Some(100),
            enable_simd: Some(true),
            request_info_callback: None,
        }
    }

    /// Create options optimized for comprehensive results
    pub fn comprehensive() -> Self {
        Self {
            limit: Some(50),
            min_similarity: Some(0.5),
            filters: None,
            include_vectors: Some(true),
            include_metadata: Some(true),
            include_rank: Some(true),
            candidate_limit: Some(10000),
            enable_simd: Some(true),
            request_info_callback: None,
        }
    }

    /// Validate the options and return normalized values
    pub fn validate(mut self) -> Result<Self> {
        // Clamp similarity threshold
        if let Some(threshold) = self.min_similarity
            && !(0.0..=1.0).contains(&threshold)
        {
            return Err(crate::memory::utils::error::Error::InvalidInput(
                "min_similarity must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Ensure reasonable limits
        if let Some(limit) = self.limit {
            if limit == 0 {
                self.limit = Some(1);
            } else if limit > 10000 {
                self.limit = Some(10000);
            }
        }

        if let Some(candidate_limit) = self.candidate_limit
            && candidate_limit == 0
        {
            self.candidate_limit = Some(100);
        }

        Ok(self)
    }
}
