//! Similarity computation types and traits
//!
//! This module provides core types and traits for computing and working with
//! similarity metrics between vectors and other data structures.

use serde::{Deserialize, Serialize};

/// Similarity metrics that can be used for comparing vectors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SimilarityMetric {
    /// Cosine similarity (normalized dot product)
    #[default]
    Cosine,
    /// Euclidean distance (L2 norm)
    Euclidean,
    /// Manhattan distance (L1 norm)
    Manhattan,
    /// Dot product (unnormalized similarity)
    DotProduct,
    /// Jaccard similarity for sets
    Jaccard,
}

/// Result of a similarity computation between two vectors or embeddings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimilarityResult {
    /// Similarity score (higher is more similar for similarity metrics,
    /// lower is more similar for distance metrics)
    pub score: f32,

    /// Whether the score exceeds the configured threshold
    pub is_similar: bool,

    /// Distance value (for distance-based metrics)
    pub distance: Option<f32>,

    /// Metric used for the computation
    pub metric: SimilarityMetric,
}

impl SimilarityResult {
    /// Create a new similarity result
    #[inline]
    #[must_use]
    pub fn new(score: f32, is_similar: bool, metric: SimilarityMetric) -> Self {
        Self {
            score,
            is_similar,
            distance: None,
            metric,
        }
    }

    /// Create a new similarity result with distance
    #[inline]
    #[must_use]
    pub fn with_distance(
        score: f32,
        is_similar: bool,
        distance: f32,
        metric: SimilarityMetric,
    ) -> Self {
        Self {
            score,
            is_similar,
            distance: Some(distance),
            metric,
        }
    }

    /// Check if the similarity meets the given threshold
    #[inline]
    #[must_use]
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        match self.metric {
            // For similarity metrics, higher is better
            SimilarityMetric::Cosine | SimilarityMetric::DotProduct | SimilarityMetric::Jaccard => {
                self.score >= threshold
            }
            // For distance metrics, lower is better
            SimilarityMetric::Euclidean | SimilarityMetric::Manhattan => self.score <= threshold,
        }
    }
}

/// Trait for types that can compute similarity between themselves and other values
pub trait Similarity<T = Self> {
    /// Compute similarity between self and other
    fn similarity(&self, other: &T, metric: SimilarityMetric) -> SimilarityResult;

    /// Check if self is similar to other based on a threshold
    fn is_similar(&self, other: &T, threshold: f32, metric: SimilarityMetric) -> bool {
        self.similarity(other, metric).meets_threshold(threshold)
    }
}
