//! Vector repository for managing vector collections

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::memory::utils::Result;
use crate::memory::vector::{DistanceMetric, VectorIndex, VectorIndexConfig, VectorIndexFactory};

/// Vector collection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCollection {
    /// Collection ID
    pub id: String,

    /// Collection name
    pub name: String,

    /// Vector dimensions
    pub dimensions: usize,

    /// Distance metric
    pub metric: DistanceMetric,

    /// Number of vectors
    pub count: usize,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Vector repository for managing multiple vector collections
pub struct VectorRepository {
    /// Collections storage
    collections: Arc<RwLock<HashMap<String, VectorCollectionHandle>>>,

    /// Default index configuration
    default_config: VectorIndexConfig,
}

/// Handle to a vector collection
struct VectorCollectionHandle {
    metadata: VectorCollection,
    index: Box<dyn VectorIndex>,
}

impl VectorRepository {
    /// Create a new vector repository
    pub fn new(default_dimensions: usize) -> Self {
        let default_config = VectorIndexConfig {
            metric: DistanceMetric::Cosine,
            dimensions: default_dimensions,
            index_type: crate::memory::vector::vector_index::IndexType::Flat,
            parameters: HashMap::new(),
        };

        Self {
            collections: Arc::new(RwLock::new(HashMap::new())),
            default_config,
        }
    }

    /// Create a new collection
    pub async fn create_collection(
        &self,
        name: String,
        dimensions: usize,
        metric: DistanceMetric,
    ) -> Result<VectorCollection> {
        let mut collections = self.collections.write().await;

        if collections.contains_key(&name) {
            return Err(crate::memory::utils::error::Error::AlreadyExists(format!(
                "Collection '{name}' already exists"
            )));
        }

        let now = crate::domain::memory::cache::get_cached_utc();
        let metadata = VectorCollection {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.clone(),
            dimensions,
            metric,
            count: 0,
            created_at: now,
            updated_at: now,
        };

        let config = VectorIndexConfig {
            metric,
            dimensions,
            ..self.default_config.clone()
        };

        let index = VectorIndexFactory::create(config);

        collections.insert(
            name,
            VectorCollectionHandle {
                metadata: metadata.clone(),
                index,
            },
        );

        Ok(metadata)
    }

    /// Delete a collection
    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        let mut collections = self.collections.write().await;

        if collections.remove(name).is_none() {
            return Err(crate::memory::utils::error::Error::NotFound(format!(
                "Collection '{name}' not found"
            )));
        }

        Ok(())
    }

    /// Get collection metadata
    pub async fn get_collection(&self, name: &str) -> Result<VectorCollection> {
        let collections = self.collections.read().await;

        collections
            .get(name)
            .map(|handle| handle.metadata.clone())
            .ok_or_else(|| {
                crate::memory::utils::error::Error::NotFound(format!(
                    "Collection '{name}' not found"
                ))
            })
    }

    /// List all collections
    pub async fn list_collections(&self) -> Result<Vec<VectorCollection>> {
        let collections = self.collections.read().await;
        Ok(collections
            .values()
            .map(|handle| handle.metadata.clone())
            .collect())
    }

    /// Add a vector to a collection
    pub async fn add_vector(
        &self,
        collection_name: &str,
        id: String,
        vector: Vec<f32>,
    ) -> Result<()> {
        let mut collections = self.collections.write().await;

        let handle = collections.get_mut(collection_name).ok_or_else(|| {
            crate::memory::utils::error::Error::NotFound(format!(
                "Collection '{collection_name}' not found"
            ))
        })?;

        handle.index.add(id, vector)?;
        handle.metadata.count = handle.index.len();
        handle.metadata.updated_at = crate::domain::memory::cache::get_cached_utc();

        Ok(())
    }

    /// Remove a vector from a collection
    pub async fn remove_vector(&self, collection_name: &str, id: &str) -> Result<()> {
        let mut collections = self.collections.write().await;

        let handle = collections.get_mut(collection_name).ok_or_else(|| {
            crate::memory::utils::error::Error::NotFound(format!(
                "Collection '{collection_name}' not found"
            ))
        })?;

        handle.index.remove(id)?;
        handle.metadata.count = handle.index.len();
        handle.metadata.updated_at = crate::domain::memory::cache::get_cached_utc();

        Ok(())
    }

    /// Search for similar vectors in a collection
    pub async fn search(
        &self,
        collection_name: &str,
        query: &[f32],
        k: usize,
    ) -> Result<Vec<(String, f32)>> {
        let collections = self.collections.read().await;

        let handle = collections.get(collection_name).ok_or_else(|| {
            crate::memory::utils::error::Error::NotFound(format!(
                "Collection '{collection_name}' not found"
            ))
        })?;

        handle.index.search(query, k)
    }

    /// Build/rebuild index for a collection
    pub async fn build_index(&self, collection_name: &str) -> Result<()> {
        let mut collections = self.collections.write().await;

        let handle = collections.get_mut(collection_name).ok_or_else(|| {
            crate::memory::utils::error::Error::NotFound(format!(
                "Collection '{collection_name}' not found"
            ))
        })?;

        handle.index.build()?;
        handle.metadata.updated_at = crate::domain::memory::cache::get_cached_utc();

        Ok(())
    }
}

/// Batch operations for vector repository
impl VectorRepository {
    /// Add multiple vectors to a collection
    pub async fn add_vectors_batch(
        &self,
        collection_name: &str,
        vectors: Vec<(String, Vec<f32>)>,
    ) -> Result<()> {
        let mut collections = self.collections.write().await;

        let handle = collections.get_mut(collection_name).ok_or_else(|| {
            crate::memory::utils::error::Error::NotFound(format!(
                "Collection '{collection_name}' not found"
            ))
        })?;

        for (id, vector) in vectors {
            handle.index.add(id, vector)?;
        }

        handle.metadata.count = handle.index.len();
        handle.metadata.updated_at = crate::domain::memory::cache::get_cached_utc();

        // Rebuild index after batch insert for better performance
        handle.index.build()?;

        Ok(())
    }

    /// Remove multiple vectors from a collection
    pub async fn remove_vectors_batch(
        &self,
        collection_name: &str,
        ids: Vec<String>,
    ) -> Result<()> {
        let mut collections = self.collections.write().await;

        let handle = collections.get_mut(collection_name).ok_or_else(|| {
            crate::memory::utils::error::Error::NotFound(format!(
                "Collection '{collection_name}' not found"
            ))
        })?;

        for id in ids {
            handle.index.remove(&id)?;
        }

        handle.metadata.count = handle.index.len();
        handle.metadata.updated_at = crate::domain::memory::cache::get_cached_utc();

        Ok(())
    }
}

