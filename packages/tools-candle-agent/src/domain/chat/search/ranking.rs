//! Search result ranking algorithms

use std::collections::HashMap;

use super::types::{ProcessedQuery, RankingAlgorithm, SearchResult};

/// Result ranker for scoring and sorting search results
pub struct ResultRanker {
    /// Ranking algorithm to use
    algorithm: RankingAlgorithm,
    /// Field boost weights
    field_boosts: HashMap<String, f32>,
}

impl ResultRanker {
    /// Create a new result ranker
    #[must_use]
    pub fn new() -> Self {
        Self {
            algorithm: RankingAlgorithm::Bm25,
            field_boosts: HashMap::new(),
        }
    }

    /// Rank search results by relevance
    ///
    /// # Errors
    ///
    /// Returns error if result ranking fails
    pub fn rank_results_sync(
        &self,
        mut results: Vec<SearchResult>,
        _query: &ProcessedQuery,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        // Sort by relevance score (descending)
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }
}

impl Default for ResultRanker {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ResultRanker {
    fn clone(&self) -> Self {
        Self {
            algorithm: self.algorithm.clone(),
            field_boosts: self.field_boosts.clone(),
        }
    }
}
