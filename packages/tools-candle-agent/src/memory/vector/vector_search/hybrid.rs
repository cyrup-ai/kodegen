//! Hybrid search combining vector and keyword strategies

use std::cmp::Ordering;
use std::collections::HashMap;

use super::core::VectorSearch;
use super::options::SearchOptions;
use super::types::{KeywordSearchFn, SearchResult};
use crate::memory::utils::error::Result;

/// Hybrid search combining vector and keyword search strategies
///
/// This implementation provides sophisticated search capabilities by combining:
/// - Vector similarity search for semantic matching
/// - Keyword search for exact term matching
/// - Configurable weighting between strategies
/// - Advanced result merging and ranking algorithms
#[derive(Clone)]
pub struct HybridSearch {
    /// Vector search component
    vector_search: VectorSearch,
    /// Keyword search function
    keyword_search: KeywordSearchFn,
    /// Weight for vector search results (0.0 to 1.0)
    vector_weight: f32,
    /// Weight for keyword search results (computed as 1.0 - vector_weight)
    keyword_weight: f32,
}

impl HybridSearch {
    /// Create a new HybridSearch with custom keyword search function
    ///
    /// # Arguments
    /// * `vector_search` - Vector search implementation
    /// * `keyword_search` - Synchronous keyword search function
    /// * `vector_weight` - Weight for vector results (0.0 to 1.0, default: 0.5)
    ///
    /// # Returns
    /// New HybridSearch instance
    pub fn new(
        vector_search: VectorSearch,
        keyword_search: KeywordSearchFn,
        vector_weight: Option<f32>,
    ) -> Self {
        let vector_weight = vector_weight.unwrap_or(0.5).clamp(0.0, 1.0);
        let keyword_weight = 1.0 - vector_weight;

        Self {
            vector_search,
            keyword_search,
            vector_weight,
            keyword_weight,
        }
    }

    /// Search using both vector and keyword strategies (synchronous)
    ///
    /// # Arguments
    /// * `text` - Search query text
    /// * `options` - Search options applied to both strategies
    ///
    /// # Returns
    /// Result containing merged and ranked search results
    pub async fn search(
        &self,
        text: &str,
        options: Option<SearchOptions>,
    ) -> Result<Vec<SearchResult>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        // Execute both searches in parallel using tokio tasks
        let vector_search = self.vector_search.clone();
        let text_for_vector = text.to_string();
        let options_for_vector = options.clone();

        let vector_handle = tokio::spawn(async move {
            vector_search
                .search_by_text(&text_for_vector, options_for_vector)
                .await
        });

        // Keyword search
        let keyword_search = self.keyword_search.clone();
        let text_for_keyword = text.to_string();
        let options_for_keyword = options.clone();

        let keyword_handle = tokio::task::spawn_blocking(move || {
            (keyword_search)(&text_for_keyword, options_for_keyword)
        });

        // Wait for both results
        let vector_results = vector_handle.await.map_err(|e| {
            crate::memory::utils::error::Error::Other(format!("Vector search task failed: {}", e))
        })??;

        let keyword_results = keyword_handle.await.map_err(|e| {
            crate::memory::utils::error::Error::Other(format!("Keyword search task failed: {}", e))
        })??;

        // Combine and rank results
        let combined_results = self.combine_results(vector_results, keyword_results, options);

        Ok(combined_results)
    }

    /// Combine vector and keyword search results with sophisticated merging
    fn combine_results(
        &self,
        vector_results: Vec<SearchResult>,
        keyword_results: Vec<SearchResult>,
        options: Option<SearchOptions>,
    ) -> Vec<SearchResult> {
        let options = options.unwrap_or_default();
        let limit = options.limit.unwrap_or(10);

        // Create a map for efficient result merging
        let mut combined_map = HashMap::new();

        // Process vector results
        for result in vector_results {
            let weighted_similarity = result.similarity * self.vector_weight;
            combined_map.insert(
                result.id.clone(),
                SearchResult {
                    id: result.id,
                    vector: result.vector,
                    similarity: result.similarity,
                    metadata: result.metadata,
                    rank: result.rank,
                    combined_score: Some(weighted_similarity),
                    decision_confidence: result.decision_confidence,
                },
            );
        }

        // Process keyword results and merge
        for result in keyword_results {
            let weighted_similarity = result.similarity * self.keyword_weight;

            if let Some(existing) = combined_map.remove(&result.id) {
                // Merge with existing vector result
                let new_combined_score =
                    existing.combined_score.unwrap_or(0.0) + weighted_similarity;

                combined_map.insert(
                    result.id.clone(),
                    SearchResult {
                        id: result.id,
                        vector: existing.vector, // Prefer vector data
                        similarity: existing.similarity,
                        metadata: existing.metadata.or(result.metadata), // Merge metadata
                        rank: None,                                      // Will be recomputed
                        combined_score: Some(new_combined_score),
                        decision_confidence: existing.decision_confidence,
                    },
                );
            } else {
                // New keyword-only result
                combined_map.insert(
                    result.id.clone(),
                    SearchResult {
                        id: result.id,
                        vector: result.vector,
                        similarity: result.similarity,
                        metadata: result.metadata,
                        rank: result.rank,
                        combined_score: Some(weighted_similarity),
                        decision_confidence: result.decision_confidence,
                    },
                );
            }
        }

        // Convert to vector and sort by combined score
        let mut combined_results: Vec<_> = combined_map.into_values().collect();

        // Sort by effective score (descending) with NaN handling
        combined_results.sort_by(|a, b| {
            let score_a = a.effective_score();
            let score_b = b.effective_score();

            match (score_a.is_nan(), score_b.is_nan()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater, // NaN goes to end
                (false, true) => Ordering::Less,    // NaN goes to end
                (false, false) => score_b.partial_cmp(&score_a).unwrap_or(Ordering::Equal),
            }
        });

        // Apply final limit and assign ranks
        if combined_results.len() > limit {
            combined_results.truncate(limit);
        }

        // Assign final rankings if requested
        if options.include_rank.unwrap_or(false) {
            for (index, result) in combined_results.iter_mut().enumerate() {
                result.rank = Some(index + 1);
            }
        }

        combined_results
    }

    /// Update search weights
    ///
    /// # Arguments
    /// * `vector_weight` - New weight for vector search (0.0 to 1.0)
    pub fn set_vector_weight(&mut self, weight: f32) {
        self.vector_weight = weight.clamp(0.0, 1.0);
        self.keyword_weight = 1.0 - self.vector_weight;
    }

    /// Get current vector weight
    pub fn vector_weight(&self) -> f32 {
        self.vector_weight
    }

    /// Get current keyword weight
    pub fn keyword_weight(&self) -> f32 {
        self.keyword_weight
    }

    /// Get reference to the vector search component
    pub fn vector_search(&self) -> &VectorSearch {
        &self.vector_search
    }
}
