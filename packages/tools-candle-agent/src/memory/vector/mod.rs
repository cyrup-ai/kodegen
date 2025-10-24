//! Vector operations and storage for memory embeddings

pub mod in_memory;
pub mod multimodal_service;
pub mod vector_index;
pub mod vector_repository;
pub mod vector_search;
pub mod vector_store;

// Re-export main types
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub use multimodal_service::MultimodalEmbeddingService;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio_stream::Stream;
use uuid;
pub use vector_index::*;
pub use vector_repository::*;
pub use vector_search::*;

/// Distance metrics for vector comparison
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceMetric {
    /// Euclidean distance (L2 norm)
    Euclidean,
    /// Cosine similarity
    Cosine,
    /// Dot product
    DotProduct,
}

/// A pending vector operation
pub struct PendingVectorOp {
    rx: oneshot::Receiver<crate::memory::utils::Result<()>>,
}

impl PendingVectorOp {
    pub fn new(rx: oneshot::Receiver<crate::memory::utils::Result<()>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingVectorOp {
    type Output = crate::memory::utils::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(crate::memory::utils::error::Error::Internal(
                "Vector operation task failed".to_string(),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A pending vector search
pub struct PendingVectorSearch {
    rx: oneshot::Receiver<crate::memory::utils::Result<Vec<VectorSearchResult>>>,
}

impl PendingVectorSearch {
    pub fn new(
        rx: oneshot::Receiver<crate::memory::utils::Result<Vec<VectorSearchResult>>>,
    ) -> Self {
        Self { rx }
    }
}

impl Future for PendingVectorSearch {
    type Output = crate::memory::utils::Result<Vec<VectorSearchResult>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(crate::memory::utils::error::Error::Internal(
                "Vector search task failed".to_string(),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A pending embedding generation
pub struct PendingEmbedding {
    rx: oneshot::Receiver<crate::memory::utils::Result<Vec<f32>>>,
}

impl PendingEmbedding {
    pub fn new(rx: oneshot::Receiver<crate::memory::utils::Result<Vec<f32>>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingEmbedding {
    type Output = crate::memory::utils::Result<Vec<f32>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(crate::memory::utils::error::Error::Internal(
                "Embedding task failed".to_string(),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Vector store trait for different implementations
pub trait VectorStore: Send + Sync {
    /// Add a vector with metadata
    fn add(
        &self,
        id: String,
        embedding: Vec<f32>,
        metadata: Option<serde_json::Value>,
    ) -> PendingVectorOp;

    /// Update a vector
    fn update(
        &self,
        id: String,
        embedding: Vec<f32>,
        metadata: Option<serde_json::Value>,
    ) -> PendingVectorOp;

    /// Delete a vector
    fn delete(&self, id: String) -> PendingVectorOp;

    /// Search for similar vectors
    fn search(
        &self,
        query: Vec<f32>,
        limit: usize,
        filter: Option<crate::memory::filter::MemoryFilter>,
    ) -> Pin<Box<dyn Stream<Item = VectorSearchResult> + Send>>;

    /// Generate embedding for text
    fn embed(&self, text: String) -> PendingEmbedding;
}

/// Vector search result
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorSearchResult {
    /// ID of the result
    pub id: String,

    /// Similarity score
    pub score: f32,

    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

impl cyrup_sugars::prelude::MessageChunk for VectorSearchResult {
    fn bad_chunk(error: String) -> Self {
        Self {
            id: format!("error-{}", uuid::Uuid::new_v4()),
            score: 0.0,
            metadata: Some(serde_json::json!({
                "error": error
            })),
        }
    }

    fn error(&self) -> Option<&str> {
        self.metadata
            .as_ref()
            .and_then(|m| m.get("error"))
            .and_then(|e| e.as_str())
    }
}
