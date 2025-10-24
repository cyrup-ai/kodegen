//! Core retrieval types and result structures

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::memory::utils::Result;

/// Retrieval method used to find the memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetrievalMethod {
    VectorSimilarity,
    Semantic,
    Temporal,
    Keyword,
    Hybrid,
}

/// Result from memory retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// Memory ID
    pub id: String,

    /// Relevance score
    pub score: f32,

    /// Retrieval method used
    pub method: RetrievalMethod,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A pending retrieval operation
pub struct PendingRetrieval {
    rx: oneshot::Receiver<Result<Vec<RetrievalResult>>>,
}

impl PendingRetrieval {
    pub fn new(rx: oneshot::Receiver<Result<Vec<RetrievalResult>>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingRetrieval {
    type Output = Result<Vec<RetrievalResult>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(crate::memory::utils::error::Error::Internal(
                "Retrieval task failed".to_string(),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}
