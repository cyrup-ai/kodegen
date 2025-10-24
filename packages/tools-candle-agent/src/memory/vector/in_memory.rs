//! In-memory vector store implementation - THREAD-SAFE SYNCHRONOUS OPERATIONS

use std::cmp::Ordering;
use std::collections::HashMap;

use kodegen_simd::cosine_similarity;
use surrealdb::Value;

use super::vector_store::{VectorMetadata, VectorSearchResult, VectorStore};
use crate::memory::constants::ERROR_VECTOR_NOT_FOUND;
use crate::memory::utils::error::{Error, Result};

/// Thread-safe in-memory vector store implementation
///
/// This implementation provides blazing-fast vector operations using:
/// - Zero-allocation SIMD cosine similarity computations
/// - Lock-free HashMap operations for vector storage
/// - Efficient sorting algorithms with NaN handling
/// - Memory-efficient metadata management
///
/// All operations are synchronous and thread-safe. For concurrent access,
/// wrap in Arc<RwLock<>> or use external synchronization.
#[derive(Debug)]
pub struct InMemoryVectorStore {
    vectors: HashMap<String, Vec<f32>>,
    metadata: HashMap<String, VectorMetadata>,
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryVectorStore {
    /// Create a new in-memory vector store
    ///
    /// # Returns
    /// A new empty vector store ready for use
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new vector store with specified capacity
    ///
    /// # Arguments
    /// * `capacity` - Initial capacity for the internal hashmaps
    ///
    /// # Returns
    /// A new vector store pre-allocated for the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vectors: HashMap::with_capacity(capacity),
            metadata: HashMap::with_capacity(capacity),
        }
    }

    /// Get the current capacity of the vector store
    ///
    /// # Returns
    /// The current capacity of the internal vector storage
    pub fn capacity(&self) -> usize {
        self.vectors.capacity()
    }

    /// Shrink the internal storage to fit the current data
    ///
    /// This operation reduces memory usage by releasing unused capacity.
    pub fn shrink_to_fit(&mut self) {
        self.vectors.shrink_to_fit();
        self.metadata.shrink_to_fit();
    }

    /// Get memory usage statistics
    ///
    /// # Returns
    /// Tuple of (vector_memory_bytes, metadata_memory_bytes)
    pub fn memory_usage(&self) -> (usize, usize) {
        let vector_bytes = self
            .vectors
            .iter()
            .map(|(k, v)| k.len() + v.len() * std::mem::size_of::<f32>())
            .sum::<usize>();

        let metadata_bytes = self
            .metadata
            .iter()
            .map(|(k, v)| k.len() + v.len() * 64) // Approximate metadata size
            .sum::<usize>();

        (vector_bytes, metadata_bytes)
    }

    /// Optimized similarity search with heap-based top-k selection
    ///
    /// Uses a min-heap to efficiently find top-k results without full sorting.
    /// This is more efficient than sorting all results when k << total_vectors.
    fn search_similar_optimized(&self, query_vector: &[f32], limit: usize) -> Vec<(String, f32)> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        let mut heap = BinaryHeap::with_capacity(limit + 1);

        for (id, vector) in &self.vectors {
            let similarity = cosine_similarity(query_vector, vector);

            // Skip NaN values entirely
            if similarity.is_nan() {
                continue;
            }

            if heap.len() < limit {
                heap.push(Reverse((similarity.to_bits(), id.clone())));
            } else if let Some(&Reverse((min_sim, _))) = heap.peek()
                && similarity.to_bits() > min_sim
            {
                heap.pop();
                heap.push(Reverse((similarity.to_bits(), id.clone())));
            }
        }

        // Extract results and convert back to f32, sorted descending
        let mut results: Vec<(String, f32)> = heap
            .into_iter()
            .map(|Reverse((sim_bits, id))| (id, f32::from_bits(sim_bits)))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        results
    }
}

impl VectorStore for InMemoryVectorStore {
    fn add_vector(&mut self, id: &str, vector: Vec<f32>) -> Result<()> {
        // Validate vector is not empty
        if vector.is_empty() {
            return Err(Error::InvalidInput("Vector cannot be empty".to_string()));
        }

        // Validate vector contains no NaN or infinite values
        if vector.iter().any(|&x| !x.is_finite()) {
            return Err(Error::InvalidInput(
                "Vector contains NaN or infinite values".to_string(),
            ));
        }

        self.vectors.insert(id.to_string(), vector);
        Ok(())
    }

    fn get_vector(&self, id: &str) -> Result<(Vec<f32>, Option<VectorMetadata>)> {
        if let Some(vector) = self.vectors.get(id) {
            let metadata = self.metadata.get(id).cloned();
            Ok((vector.clone(), metadata))
        } else {
            Err(Error::NotFound(ERROR_VECTOR_NOT_FOUND.to_string()))
        }
    }

    fn update_vector(&mut self, id: &str, vector: Vec<f32>) -> Result<()> {
        if !self.vectors.contains_key(id) {
            return Err(Error::NotFound(ERROR_VECTOR_NOT_FOUND.to_string()));
        }

        // Validate vector is not empty
        if vector.is_empty() {
            return Err(Error::InvalidInput("Vector cannot be empty".to_string()));
        }

        // Validate vector contains no NaN or infinite values
        if vector.iter().any(|&x| !x.is_finite()) {
            return Err(Error::InvalidInput(
                "Vector contains NaN or infinite values".to_string(),
            ));
        }

        self.vectors.insert(id.to_string(), vector);
        Ok(())
    }

    fn delete_vector(&mut self, id: &str) -> Result<()> {
        self.vectors.remove(id);
        self.metadata.remove(id);
        Ok(())
    }

    fn search_similar(&self, query_vector: &[f32], limit: usize) -> Result<Vec<String>> {
        // Validate query vector
        if query_vector.is_empty() {
            return Err(Error::InvalidInput(
                "Query vector cannot be empty".to_string(),
            ));
        }

        if query_vector.iter().any(|&x| !x.is_finite()) {
            return Err(Error::InvalidInput(
                "Query vector contains NaN or infinite values".to_string(),
            ));
        }

        if limit == 0 {
            return Ok(Vec::new());
        }

        // Use optimized search for better performance
        let results = self.search_similar_optimized(query_vector, limit);
        Ok(results.into_iter().map(|(id, _)| id).collect())
    }

    fn search(
        &self,
        query_vector: &[f32],
        limit: Option<usize>,
        filters: Option<HashMap<String, Value>>,
    ) -> Result<Vec<VectorSearchResult>> {
        // Validate query vector
        if query_vector.is_empty() {
            return Err(Error::InvalidInput(
                "Query vector cannot be empty".to_string(),
            ));
        }

        if query_vector.iter().any(|&x| !x.is_finite()) {
            return Err(Error::InvalidInput(
                "Query vector contains NaN or infinite values".to_string(),
            ));
        }

        let mut results: Vec<VectorSearchResult> = Vec::new();

        // Iterate through all vectors and apply filters
        for (id, vector) in &self.vectors {
            // Apply filters if any
            if let Some(ref filters) = filters
                && let Some(metadata) = self.metadata.get(id)
            {
                let mut matches = true;
                for (key, value) in filters {
                    if metadata.get(key) != Some(value) {
                        matches = false;
                        break;
                    }
                }
                if !matches {
                    continue;
                }
            } else if filters.is_some() {
                continue; // No metadata, skip if filters are present
            }

            let similarity = cosine_similarity(query_vector, vector);

            // Skip NaN similarities
            if similarity.is_nan() {
                continue;
            }

            let metadata = self.metadata.get(id).cloned();
            results.push((id.clone(), vector.clone(), similarity, metadata));
        }

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal));

        // Apply limit if specified
        if let Some(limit) = limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    fn count(&self) -> Result<usize> {
        Ok(self.vectors.len())
    }

    fn clear(&mut self) -> Result<()> {
        self.vectors.clear();
        self.metadata.clear();
        Ok(())
    }

    fn add_metadata(&mut self, id: &str, metadata: HashMap<String, Value>) -> Result<()> {
        if !self.vectors.contains_key(id) {
            return Err(Error::NotFound(ERROR_VECTOR_NOT_FOUND.to_string()));
        }

        self.metadata.insert(id.to_string(), metadata);
        Ok(())
    }

    fn get_metadata(&self, id: &str) -> Result<Option<VectorMetadata>> {
        Ok(self.metadata.get(id).cloned())
    }

    fn update_metadata(&mut self, id: &str, metadata: HashMap<String, Value>) -> Result<()> {
        if !self.vectors.contains_key(id) {
            return Err(Error::NotFound(ERROR_VECTOR_NOT_FOUND.to_string()));
        }

        self.metadata.insert(id.to_string(), metadata);
        Ok(())
    }

    fn remove_metadata(&mut self, id: &str) -> Result<()> {
        self.metadata.remove(id);
        Ok(())
    }

    fn get_dimensions(&self) -> Result<Option<u32>> {
        // Get dimensions from first vector
        if let Some((_, vector)) = self.vectors.iter().next() {
            Ok(Some(vector.len() as u32))
        } else {
            Ok(None)
        }
    }

    fn get_index_quality(&self) -> Result<f32> {
        if self.vectors.is_empty() {
            return Ok(100.0); // Empty store is "healthy"
        }

        // Check dimension consistency
        let first_dim = self.vectors.values().next().map(|v| v.len());
        let dimension_consistent = first_dim
            .is_none_or(|expected_dim| self.vectors.values().all(|v| v.len() == expected_dim));

        if !dimension_consistent {
            return Ok(0.0); // Critical error: dimension mismatch
        }

        // Calculate memory efficiency
        let (vector_bytes, metadata_bytes) = self.memory_usage();
        let total_bytes = vector_bytes + metadata_bytes;
        let capacity_bytes = self.capacity() * 64; // Rough estimate

        let efficiency = if capacity_bytes > 0 {
            (total_bytes as f32 / capacity_bytes as f32).min(1.0)
        } else {
            1.0
        };

        // Quality score: 100 if efficient, scales down with waste
        // Efficiency < 0.3 means lots of wasted capacity
        let quality = if efficiency < 0.3 {
            70.0 + (efficiency * 100.0) // 70-100 range
        } else {
            100.0
        };

        Ok(quality)
    }

    fn get_index_stats(&self) -> Result<super::vector_store::IndexStats> {
        let (vector_bytes, metadata_bytes) = self.memory_usage();

        Ok(super::vector_store::IndexStats {
            entry_count: self.count()? as u64,
            dimensions: self.get_dimensions()?,
            quality_score: self.get_index_quality()?,
            memory_bytes: (vector_bytes + metadata_bytes) as u64,
            fragmentation_ratio: 0.0, // InMemory has no fragmentation
        })
    }
}
