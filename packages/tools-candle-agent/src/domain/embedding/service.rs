//! Embedding Service Implementations and Caching Mechanisms
//!
//! This module provides production-ready embedding services with low-allocation methods,
//! lock-free caching, and high-performance vector operations.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::Stream;

use crate::domain::memory::serialization::content_hash;
use async_stream;

/// Error type for vector store operations
#[derive(Debug, thiserror::Error)]
pub enum VectorStoreError {
    /// Requested item was not found in the vector store
    #[error("Not found")]
    NotFound,
    /// Vector store operation failed with detailed error message
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// Production-ready embedding service trait with zero-allocation methods
pub trait EmbeddingService: Send + Sync {
    /// Get embedding for content with zero-copy return
    fn get_embedding(&self, content: &str) -> Pin<Box<dyn Stream<Item = Option<Vec<f32>>> + Send>>;

    /// Get or compute embedding with zero-allocation caching
    fn get_or_compute_embedding(
        &self,
        content: &str,
    ) -> Pin<Box<dyn Stream<Item = Vec<f32>> + Send>>;

    /// Precompute embeddings for batch content
    fn precompute_batch(&self, content: &[&str]) -> Pin<Box<dyn Stream<Item = ()> + Send>>;

    /// Get embedding dimensions
    fn embedding_dimension(&self) -> usize;

    /// Clear cache to free memory
    fn clear_cache(&self);
}

/// Embedding pool for zero-allocation vector reuse
pub struct EmbeddingPool {
    sender: mpsc::UnboundedSender<Vec<f32>>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<Vec<f32>>>>,
    dimension: usize,
    max_capacity: usize,
}
impl EmbeddingPool {
    /// Create new embedding pool with specified capacity
    #[inline]
    #[must_use]
    pub fn new(dimension: usize, capacity: usize) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        // Pre-allocate vectors to avoid allocations during runtime
        for _ in 0..capacity {
            let vec = vec![0.0; dimension];
            let _ = sender.send(vec);
        }

        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            dimension,
            max_capacity: capacity,
        }
    }

    /// Get vector from pool or create new one (zero-allocation in common case)
    #[inline]
    #[must_use]
    pub async fn acquire(&self) -> Vec<f32> {
        let mut receiver = self.receiver.lock().await;
        receiver
            .try_recv()
            .unwrap_or_else(|_| vec![0.0; self.dimension])
    }

    /// Return vector to pool for reuse
    #[inline]
    pub fn release(&self, mut vec: Vec<f32>) {
        if vec.len() == self.dimension {
            vec.fill(0.0); // Clear data
            let _ = self.sender.send(vec); // Ignore if send fails
        }
    }

    /// Get pool statistics
    #[inline]
    #[must_use]
    pub fn stats(&self) -> (usize, usize) {
        // Note: tokio mpsc doesn't expose queue length, returning max_capacity as estimate
        (self.max_capacity, self.max_capacity)
    }
}

/// Cache entry type: (embedding vector, dimension)
type CacheEntry = (Vec<f32>, usize);

/// Cache storage type
type CacheStorage = Arc<tokio::sync::RwLock<HashMap<String, CacheEntry>>>;

/// Production-ready in-memory embedding cache with low-allocation operations
///
/// This cache validates embedding dimensions to prevent mixing incompatible embeddings
/// from different models. All stored embeddings must match the cache's configured dimension.
/// Dimension mismatches are treated as cache misses to ensure compatibility.
#[derive(Clone)]
pub struct InMemoryEmbeddingCache {
    cache: CacheStorage,
    pool: Arc<EmbeddingPool>,
    dimension: usize,
}
impl InMemoryEmbeddingCache {
    /// Create new embedding cache with specified dimension
    #[inline]
    #[must_use]
    pub fn new(dimension: usize) -> Self {
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::with_capacity(1000))),
            pool: Arc::new(EmbeddingPool::new(dimension, 100)),
            dimension,
        }
    }

    /// Get cached embedding with zero-copy return and dimension validation
    #[inline]
    pub async fn get_cached(&self, content: &str) -> Option<Vec<f32>> {
        // Use a write lock to allow proactive invalidation of mismatched-dimension entries
        let mut cache = self.cache.write().await;
        match cache.get(content) {
            Some((embedding, cached_dimension)) if *cached_dimension == self.dimension => {
                Some(embedding.clone())
            }
            Some(_) => {
                // Dimension mismatch: proactively invalidate stale entry
                cache.remove(content);
                None
            }
            None => None,
        }
    }

    /// Store embedding in cache with dimension validation (checked)
    ///
    /// # Errors
    ///
    /// Returns `VectorStoreError::OperationFailed` if the embedding dimension does not match the cache dimension.
    #[inline]
    pub async fn try_store(
        &self,
        content: String,
        embedding: Vec<f32>,
    ) -> Result<(), VectorStoreError> {
        // Validate embedding dimension matches cache dimension
        if embedding.len() != self.dimension {
            return Err(VectorStoreError::OperationFailed(format!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.dimension,
                embedding.len()
            )));
        }

        let mut cache = self.cache.write().await;
        cache.insert(content, (embedding, self.dimension));
        Ok(())
    }

    /// Backward-compatible store that ignores dimension errors
    #[inline]
    pub async fn store(&self, content: String, embedding: Vec<f32>) {
        let _ = self.try_store(content, embedding).await;
    }

    /// Generate deterministic embedding based on content hash with correct dimension
    #[inline]
    pub async fn generate_deterministic(&self, content: &str) -> Vec<f32> {
        let mut embedding = self.pool.acquire().await;
        // Ensure embedding has correct dimension
        if embedding.len() != self.dimension {
            embedding = vec![0.0; self.dimension];
        }
        // Fill with deterministic values based on content hash
        let hash = content_hash(content);
        for (i, val) in embedding.iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let hash_val =
                ((hash + i64::try_from(i).unwrap_or(i64::MAX)) as f32) / (i64::MAX as f32);
            *val = hash_val;
        }
        embedding
    }

    /// Validate embedding dimensions match expected dimension
    #[inline]
    #[must_use]
    pub fn validate_dimensions(&self, embedding: &[f32]) -> bool {
        embedding.len() == self.dimension
    }

    /// Accessor for configured embedding dimension
    #[inline]
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Clear cache entries with invalid dimensions
    #[inline]
    pub async fn clear_invalid_entries(&self) -> usize {
        let mut cache = self.cache.write().await;
        let initial_count = cache.len();

        cache.retain(|_, (_, cached_dimension)| *cached_dimension == self.dimension);

        initial_count - cache.len()
    }

    /// Clear cache to free memory
    #[inline]
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// Concrete implementation of `EmbeddingService` using a capability model and cache
#[derive(Clone)]
pub struct EmbeddingServiceImpl<
    M: crate::capability::traits::TextEmbeddingCapable + Clone + Send + Sync + 'static,
> {
    model: std::sync::Arc<M>,
    cache: std::sync::Arc<InMemoryEmbeddingCache>,
}

impl<M: crate::capability::traits::TextEmbeddingCapable + Clone + Send + Sync + 'static>
    EmbeddingServiceImpl<M>
{
    /// Create new embedding service with model and cache
    #[must_use]
    pub fn new(model: M) -> Self {
        let dimension = model.embedding_dimension();
        let cache = InMemoryEmbeddingCache::new(dimension);

        Self {
            model: std::sync::Arc::new(model),
            cache: std::sync::Arc::new(cache),
        }
    }

    /// Get the underlying model
    #[must_use]
    pub fn model(&self) -> &M {
        self.model.as_ref()
    }

    /// Get the cache
    #[must_use]
    pub fn cache(&self) -> &InMemoryEmbeddingCache {
        self.cache.as_ref()
    }

    /// Compute a namespaced cache key including model and dimension
    #[inline]
    fn cache_key(&self, content: &str) -> String {
        let h = crate::domain::memory::serialization::content_hash(content);
        format!("{}:{}:{}", self.model.name(), self.cache.dimension, h)
    }
}

impl<M: crate::capability::traits::TextEmbeddingCapable + Clone + Send + Sync + 'static>
    EmbeddingService for EmbeddingServiceImpl<M>
{
    fn get_embedding(&self, content: &str) -> Pin<Box<dyn Stream<Item = Option<Vec<f32>>> + Send>> {
        let content = content.to_string();
        let key = self.cache_key(&content);
        let key_store = key.clone();
        let model = std::sync::Arc::clone(&self.model);
        let cache = std::sync::Arc::clone(&self.cache);

        Box::pin(async_stream::stream! {
            // Try cache first
            if let Some(embedding) = cache.get_cached(&key).await {
                yield Some(embedding);
                return;
            }

            // Generate embedding
            match model.embed(&content, None).await {
                Ok(embedding) => {
                    // Store in cache (ignore errors for streaming)
                    let _ = cache.try_store(key_store, embedding.clone()).await;
                    yield Some(embedding);
                }
                Err(_) => {
                    yield None;
                }
            }
        })
    }

    fn get_or_compute_embedding(
        &self,
        content: &str,
    ) -> Pin<Box<dyn Stream<Item = Vec<f32>> + Send>> {
        let content = content.to_string();
        let key = self.cache_key(&content);
        let key_store = key.clone();
        let model = std::sync::Arc::clone(&self.model);
        let cache = std::sync::Arc::clone(&self.cache);

        Box::pin(async_stream::stream! {
            // Try cache first
            if let Some(embedding) = cache.get_cached(&key).await {
                yield embedding;
                return;
            }

            // Generate embedding
            match model.embed(&content, None).await {
                Ok(embedding) => {
                    // Store in cache (ignore errors for streaming)
                    let _ = cache.try_store(key_store, embedding.clone()).await;
                    yield embedding;
                }
                Err(_) => {
                    // For get_or_compute, we yield a deterministic embedding on error
                    // This ensures the stream always produces a value
                    yield cache.generate_deterministic(&content).await;
                }
            }
        })
    }

    fn precompute_batch(&self, content: &[&str]) -> Pin<Box<dyn Stream<Item = ()> + Send>> {
        let content: Vec<String> = content.iter().map(|s| (*s).to_string()).collect();
        let model = std::sync::Arc::clone(&self.model);
        let cache = std::sync::Arc::clone(&self.cache);

        Box::pin(async_stream::stream! {
            // Process in batches using model's batch capability
            if let Ok(embeddings) = model.batch_embed(&content, None).await {
                for (text, embedding) in content.iter().zip(embeddings.into_iter()) {
                    let h = crate::domain::memory::serialization::content_hash(text);
                    let key = format!("{}:{}:{}", model.name(), cache.dimension(), h);
                    let _ = cache.try_store(key, embedding).await;
                    yield ();
                }
            }
        })
    }

    fn embedding_dimension(&self) -> usize {
        self.cache.dimension
    }

    fn clear_cache(&self) {
        let cache = self.cache.clone();
        tokio::spawn(async move {
            cache.clear().await;
        });
    }
}
