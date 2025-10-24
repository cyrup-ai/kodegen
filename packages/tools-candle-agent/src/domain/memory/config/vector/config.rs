use serde::{Deserialize, Serialize};

use super::super::shared::EmbeddingConfig;
use super::index::IndexConfig;
use super::performance::{MemoryConfig, PerformanceConfig, VectorConnectionConfig};
use super::simd::SimdConfig;
use super::types::{DistanceMetric, VectorStoreType};
use crate::domain::memory::primitives::types::{MemoryError, MemoryResult};

/// Vector store configuration with SIMD optimization settings
///
/// Features:
/// - Dimension validation with compile-time checks where possible
/// - Distance metric selection with SIMD-optimized implementations
/// - Index parameters with memory usage estimation
/// - Similarity threshold with floating-point precision handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// Vector store type
    pub store_type: VectorStoreType,
    /// Embedding model configuration
    pub embedding_config: EmbeddingConfig,
    /// Vector dimension (validated at runtime)
    pub dimension: usize,
    /// Distance metric for similarity calculations
    pub distance_metric: DistanceMetric,
    /// Index configuration for performance optimization
    pub index_config: IndexConfig,
    /// SIMD optimization settings
    pub simd_config: SimdConfig,
    /// Connection configuration for external stores
    pub connection_config: Option<VectorConnectionConfig>,
    /// Performance tuning parameters
    pub performance_config: PerformanceConfig,
    /// Memory usage configuration
    pub memory_config: MemoryConfig,
}

impl VectorStoreConfig {
    /// Create new vector store configuration with validation
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if:
    /// - `dimension` is 0 or exceeds 65536
    /// - `dimension` doesn't match `embedding_config` dimension
    pub fn new(
        store_type: VectorStoreType,
        embedding_config: EmbeddingConfig,
        dimension: usize,
    ) -> MemoryResult<Self> {
        // Validate dimension
        if dimension == 0 {
            return Err(MemoryError::invalid_content(
                "Vector dimension must be greater than 0",
            ));
        }

        if dimension > 65536 {
            return Err(MemoryError::invalid_content(
                "Vector dimension must be <= 65536",
            ));
        }

        // Validate dimension matches embedding config
        if dimension != embedding_config.dimension {
            return Err(MemoryError::invalid_content(
                "Vector dimension must match embedding configuration dimension",
            ));
        }

        Ok(Self {
            store_type,
            embedding_config,
            dimension,
            distance_metric: DistanceMetric::Cosine,
            index_config: IndexConfig::optimized(
                store_type.recommended_index_type(),
                dimension,
                10000,
            ),
            simd_config: SimdConfig::optimized(),
            connection_config: None,
            performance_config: PerformanceConfig::optimized(store_type),
            memory_config: MemoryConfig::default(),
        })
    }

    /// Set distance metric
    #[must_use]
    #[inline]
    pub fn with_distance_metric(mut self, metric: DistanceMetric) -> Self {
        self.distance_metric = metric;
        self
    }

    /// Set index configuration
    #[must_use]
    #[inline]
    pub fn with_index_config(mut self, config: IndexConfig) -> Self {
        self.index_config = config;
        self
    }

    /// Set SIMD configuration
    #[must_use]
    #[inline]
    pub fn with_simd_config(mut self, config: SimdConfig) -> Self {
        self.simd_config = config;
        self
    }

    /// Set connection configuration
    #[must_use]
    #[inline]
    pub fn with_connection_config(mut self, config: VectorConnectionConfig) -> Self {
        self.connection_config = Some(config);
        self
    }

    /// Set performance configuration
    #[must_use]
    #[inline]
    pub fn with_performance_config(mut self, config: PerformanceConfig) -> Self {
        self.performance_config = config;
        self
    }

    /// Estimate memory usage for given number of vectors
    #[must_use]
    pub fn estimate_memory_usage(&self, num_vectors: usize) -> usize {
        self.index_config
            .estimate_memory_usage(self.dimension, num_vectors)
    }

    /// Check if configuration supports SIMD operations
    #[inline]
    #[must_use]
    pub fn supports_simd(&self) -> bool {
        self.distance_metric.supports_simd() && self.simd_config.should_use_simd(self.dimension)
    }

    /// Get optimal batch size for operations
    #[inline]
    #[must_use]
    pub fn optimal_batch_size(&self) -> usize {
        self.performance_config.batch_size
    }

    /// Validate configuration consistency
    ///
    /// # Errors
    ///
    /// Returns `MemoryError` if:
    /// - SIMD is enabled for a distance metric that doesn't support it
    /// - Index type is incompatible with store type
    pub fn validate(&self) -> MemoryResult<()> {
        // Check SIMD compatibility
        if self.simd_config.enable_distance_simd && !self.distance_metric.supports_simd() {
            return Err(MemoryError::validation(
                "Distance metric does not support SIMD optimization",
            ));
        }

        // Check index type compatibility with store type
        match (self.store_type, self.index_config.index_type) {
            (VectorStoreType::Memory, super::types::IndexType::IVFPQ) => {
                return Err(MemoryError::validation(
                    "Memory store does not support IVFPQ index",
                ));
            }
            (VectorStoreType::HNSW, index_type) if index_type != super::types::IndexType::HNSW => {
                return Err(MemoryError::validation(
                    "HNSW store only supports HNSW index type",
                ));
            }
            _ => {}
        }

        Ok(())
    }
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        let embedding_config = EmbeddingConfig::default();
        let dimension = embedding_config.dimension;
        let store_type = VectorStoreType::Memory;

        // Construct directly to avoid fallible new() method
        // This mirrors the logic in new() but without validation since we control the inputs
        Self {
            store_type,
            embedding_config,
            dimension,
            distance_metric: DistanceMetric::Cosine,
            index_config: IndexConfig::optimized(
                store_type.recommended_index_type(),
                dimension,
                10000,
            ),
            simd_config: SimdConfig::optimized(),
            connection_config: None,
            performance_config: PerformanceConfig::optimized(store_type),
            memory_config: MemoryConfig::default(),
        }
    }
}
