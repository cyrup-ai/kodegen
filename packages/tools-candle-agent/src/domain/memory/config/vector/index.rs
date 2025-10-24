use serde::{Deserialize, Serialize};

use super::types::IndexType;

/// Integer square root to avoid f64 cast precision/sign loss
#[inline]
const fn integer_sqrt(n: usize) -> usize {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = usize::midpoint(x, n / x);
    }
    x
}

/// Integer log2 to avoid f64 cast precision/sign loss
#[inline]
const fn integer_log2(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    usize::BITS as usize - n.leading_zeros() as usize - 1
}

/// Index configuration for performance optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Index type
    pub index_type: IndexType,
    /// Number of clusters for IVF-based indices
    pub num_clusters: Option<usize>,
    /// Number of bits for PQ compression
    pub pq_bits: Option<u8>,
    /// Number of subspaces for PQ
    pub pq_subspaces: Option<usize>,
    /// HNSW maximum connections per node
    pub hnsw_max_connections: Option<usize>,
    /// HNSW construction parameter
    pub hnsw_ef_construction: Option<usize>,
    /// Search parameter for approximate indices
    pub search_ef: Option<usize>,
    /// Number of trees for Annoy
    pub annoy_trees: Option<usize>,
}

impl IndexConfig {
    /// Create optimized configuration for index type
    #[must_use]
    pub fn optimized(index_type: IndexType, dimension: usize, expected_vectors: usize) -> Self {
        match index_type {
            IndexType::FlatIP | IndexType::FlatL2 => Self {
                index_type,
                num_clusters: None,
                pq_bits: None,
                pq_subspaces: None,
                hnsw_max_connections: None,
                hnsw_ef_construction: None,
                search_ef: None,
                annoy_trees: None,
            },
            IndexType::IVFPQ => {
                // Use integer sqrt to avoid f64 cast precision/sign loss
                let num_clusters = integer_sqrt(expected_vectors).max(1);
                let pq_subspaces = (dimension / 4).clamp(1, 64);
                Self {
                    index_type,
                    num_clusters: Some(num_clusters),
                    pq_bits: Some(8),
                    pq_subspaces: Some(pq_subspaces),
                    hnsw_max_connections: None,
                    hnsw_ef_construction: None,
                    search_ef: Some(num_clusters / 4),
                    annoy_trees: None,
                }
            }
            IndexType::HNSW => Self {
                index_type,
                num_clusters: None,
                pq_bits: None,
                pq_subspaces: None,
                hnsw_max_connections: Some(16),
                hnsw_ef_construction: Some(200),
                search_ef: Some(50),
                annoy_trees: None,
            },
            IndexType::Annoy => Self {
                index_type,
                num_clusters: None,
                pq_bits: None,
                pq_subspaces: None,
                hnsw_max_connections: None,
                hnsw_ef_construction: None,
                search_ef: None,
                annoy_trees: Some(integer_log2(expected_vectors).max(1) * 2),
            },
            IndexType::LSH | IndexType::SQ => Self {
                index_type,
                num_clusters: None,
                pq_bits: Some(8),
                pq_subspaces: None,
                hnsw_max_connections: None,
                hnsw_ef_construction: None,
                search_ef: None,
                annoy_trees: None,
            },
        }
    }

    /// Estimate memory usage in bytes
    #[must_use]
    pub fn estimate_memory_usage(&self, dimension: usize, num_vectors: usize) -> usize {
        let base_size = num_vectors * dimension * 4; // f32 vectors
        // Use const match to convert f32 multiplier to integer ratio
        // This avoids cast_sign_loss by using compile-time known positive values
        let multiplier_int = match self.index_type {
            IndexType::FlatIP | IndexType::FlatL2 => 1000, // 1.0
            IndexType::IVFPQ => 100,                       // 0.1
            IndexType::HNSW => 1500,                       // 1.5
            IndexType::Annoy => 2000,                      // 2.0
            IndexType::LSH => 1200,                        // 1.2
            IndexType::SQ => 250,                          // 0.25
        };
        base_size.saturating_mul(multiplier_int) / 1000
    }
}

impl Default for IndexConfig {
    #[inline]
    fn default() -> Self {
        Self::optimized(IndexType::FlatIP, 768, 10000)
    }
}
