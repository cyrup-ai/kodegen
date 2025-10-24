//! Vector store interface for storing and retrieving vectors - THREAD-BASED ARCHITECTURE

use std::collections::HashMap;

use surrealdb::Value;

use crate::memory::utils::error::Result;

/// Type alias for vector metadata
pub type VectorMetadata = HashMap<String, Value>;

/// Type alias for vector search results
pub type VectorSearchResult = (String, Vec<f32>, f32, Option<VectorMetadata>);

/// Index statistics for health monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexStats {
    /// Total number of entries
    pub entry_count: u64,

    /// Vector dimensions (None if empty)
    pub dimensions: Option<u32>,

    /// Quality score (0.0-100.0)
    pub quality_score: f32,

    /// Total memory usage in bytes
    pub memory_bytes: u64,

    /// Fragmentation ratio (0.0-1.0, 0.0 = no fragmentation)
    pub fragmentation_ratio: f32,
}

/// Trait for vector store implementations - SYNCHRONOUS OPERATIONS ONLY
///
/// This trait provides a thread-safe, synchronous interface for vector storage operations.
/// All implementations must be thread-safe (Send + Sync) but use synchronous methods.
/// For concurrent operations, use thread pools and channels externally.
pub trait VectorStore: Send + Sync + std::fmt::Debug {
    /// Add a vector to the store
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the vector
    /// * `vector` - Vector data to store
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    fn add_vector(&mut self, id: &str, vector: Vec<f32>) -> Result<()>;

    /// Get a vector by ID
    ///
    /// # Arguments  
    /// * `id` - Unique identifier of the vector to retrieve
    ///
    /// # Returns
    /// Result containing the vector data and optional metadata
    fn get_vector(&self, id: &str) -> Result<(Vec<f32>, Option<VectorMetadata>)>;

    /// Update a vector
    ///
    /// # Arguments
    /// * `id` - Unique identifier of the vector to update
    /// * `vector` - New vector data
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    fn update_vector(&mut self, id: &str, vector: Vec<f32>) -> Result<()>;

    /// Delete a vector
    ///
    /// # Arguments
    /// * `id` - Unique identifier of the vector to delete
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    fn delete_vector(&mut self, id: &str) -> Result<()>;

    /// Search for similar vectors
    ///
    /// # Arguments
    /// * `query_vector` - Vector to find similar vectors for
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// Result containing vector IDs sorted by similarity (descending)
    fn search_similar(&self, query_vector: &[f32], limit: usize) -> Result<Vec<String>>;

    /// Advanced search with filtering
    ///
    /// # Arguments
    /// * `query_vector` - Vector to find similar vectors for
    /// * `limit` - Optional maximum number of results
    /// * `filters` - Optional metadata filters to apply
    ///
    /// # Returns
    /// Result containing detailed search results with similarity scores
    fn search(
        &self,
        query_vector: &[f32],
        limit: Option<usize>,
        filters: Option<HashMap<String, Value>>,
    ) -> Result<Vec<VectorSearchResult>>;

    /// Get total count of vectors in the store
    ///
    /// # Returns
    /// Result containing the number of vectors stored
    fn count(&self) -> Result<usize>;

    /// Clear all vectors from the store
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    fn clear(&mut self) -> Result<()>;

    /// Add metadata for a vector
    ///
    /// # Arguments
    /// * `id` - Unique identifier of the vector
    /// * `metadata` - Metadata to associate with the vector
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    fn add_metadata(&mut self, _id: &str, _metadata: HashMap<String, Value>) -> Result<()> {
        // Default implementation returns error - implementations should override
        Err(crate::memory::utils::error::Error::Other(
            "Metadata operations not supported by this vector store".to_string(),
        ))
    }

    /// Get metadata for a vector
    ///
    /// # Arguments
    /// * `id` - Unique identifier of the vector
    ///
    /// # Returns
    /// Result containing the metadata if it exists
    fn get_metadata(&self, _id: &str) -> Result<Option<VectorMetadata>> {
        // Default implementation returns None
        Ok(None)
    }

    /// Update metadata for a vector
    ///
    /// # Arguments
    /// * `id` - Unique identifier of the vector
    /// * `metadata` - New metadata to replace existing metadata
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    fn update_metadata(&mut self, _id: &str, _metadata: HashMap<String, Value>) -> Result<()> {
        // Default implementation returns error - implementations should override
        Err(crate::memory::utils::error::Error::Other(
            "Metadata operations not supported by this vector store".to_string(),
        ))
    }

    /// Remove metadata for a vector
    ///
    /// # Arguments
    /// * `id` - Unique identifier of the vector
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    fn remove_metadata(&mut self, _id: &str) -> Result<()> {
        // Default implementation returns error - implementations should override
        Err(crate::memory::utils::error::Error::Other(
            "Metadata operations not supported by this vector store".to_string(),
        ))
    }

    /// Get index quality metrics
    ///
    /// # Returns
    /// Result containing index quality score (0.0-100.0)
    /// - 100.0 = perfect health
    /// - 80-99 = good health
    /// - 60-79 = degraded
    /// - <60 = unhealthy
    fn get_index_quality(&self) -> Result<f32> {
        // Default implementation for stores without quality tracking
        Ok(100.0)
    }

    /// Get vector dimensions
    ///
    /// # Returns
    /// Result containing the dimension size, or None if store is empty
    fn get_dimensions(&self) -> Result<Option<u32>> {
        // Default implementation returns None
        Ok(None)
    }

    /// Get index statistics
    ///
    /// # Returns
    /// Result containing detailed index statistics
    fn get_index_stats(&self) -> Result<IndexStats> {
        Ok(IndexStats {
            entry_count: self.count()? as u64,
            dimensions: self.get_dimensions()?,
            quality_score: self.get_index_quality()?,
            memory_bytes: 0, // Override in implementations
            fragmentation_ratio: 0.0,
        })
    }
}
