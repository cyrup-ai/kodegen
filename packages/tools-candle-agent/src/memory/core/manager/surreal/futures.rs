//! Future and Stream wrapper types for async memory operations.
//!
//! This module provides async primitives that wrap tokio channels for
//! memory operations, implementing the Future and Stream traits.

use std::future::Future;
use std::pin::Pin;

use crate::domain::memory::cognitive::types::CognitiveState;
use crate::memory::primitives::{MemoryNode, MemoryRelationship};
use crate::memory::utils::error::Error;

use super::Result;

/// A pending memory operation that resolves to a MemoryNode
pub struct PendingMemory {
    rx: tokio::sync::oneshot::Receiver<Result<MemoryNode>>,
}

impl PendingMemory {
    pub fn new(rx: tokio::sync::oneshot::Receiver<Result<MemoryNode>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingMemory {
    type Output = Result<MemoryNode>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// A query for a specific memory
pub struct MemoryQuery {
    rx: tokio::sync::oneshot::Receiver<Result<Option<MemoryNode>>>,
}

impl MemoryQuery {
    pub fn new(rx: tokio::sync::oneshot::Receiver<Result<Option<MemoryNode>>>) -> Self {
        Self { rx }
    }
}

impl Future for MemoryQuery {
    type Output = Result<Option<MemoryNode>>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// A pending deletion operation
pub struct PendingDeletion {
    rx: tokio::sync::oneshot::Receiver<Result<bool>>,
}

impl PendingDeletion {
    pub(super) fn new(rx: tokio::sync::oneshot::Receiver<Result<bool>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingDeletion {
    type Output = Result<bool>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// A pending relationship operation
pub struct PendingRelationship {
    rx: tokio::sync::oneshot::Receiver<Result<MemoryRelationship>>,
}

impl PendingRelationship {
    pub(super) fn new(rx: tokio::sync::oneshot::Receiver<Result<MemoryRelationship>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingRelationship {
    type Output = Result<MemoryRelationship>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Pending quantum signature update operation
pub struct PendingQuantumUpdate {
    rx: tokio::sync::oneshot::Receiver<Result<()>>,
}

impl PendingQuantumUpdate {
    pub fn new(rx: tokio::sync::oneshot::Receiver<Result<()>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingQuantumUpdate {
    type Output = Result<()>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Pending quantum signature retrieval operation
pub struct PendingQuantumSignature {
    rx: tokio::sync::oneshot::Receiver<Result<Option<CognitiveState>>>,
}

impl PendingQuantumSignature {
    pub fn new(rx: tokio::sync::oneshot::Receiver<Result<Option<CognitiveState>>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingQuantumSignature {
    type Output = Result<Option<CognitiveState>>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Pending entanglement edge creation (RELATE operation)
pub struct PendingEntanglementEdge {
    rx: tokio::sync::oneshot::Receiver<Result<()>>,
}

impl PendingEntanglementEdge {
    pub fn new(rx: tokio::sync::oneshot::Receiver<Result<()>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingEntanglementEdge {
    type Output = Result<()>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Pending single embedding generation operation
pub struct PendingEmbedding {
    rx: tokio::sync::oneshot::Receiver<Result<Vec<f32>>>,
}

impl PendingEmbedding {
    pub fn new(rx: tokio::sync::oneshot::Receiver<Result<Vec<f32>>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingEmbedding {
    type Output = Result<Vec<f32>>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Pending batch embedding generation operation
pub struct PendingBatchEmbedding {
    rx: tokio::sync::oneshot::Receiver<Result<Vec<Vec<f32>>>>,
}

impl PendingBatchEmbedding {
    pub fn new(rx: tokio::sync::oneshot::Receiver<Result<Vec<Vec<f32>>>>) -> Self {
        Self { rx }
    }
}

impl Future for PendingBatchEmbedding {
    type Output = Result<Vec<Vec<f32>>>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(_)) => {
                std::task::Poll::Ready(Err(Error::Other("Channel closed".to_string())))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// A stream of memory nodes
pub struct MemoryStream {
    rx: tokio::sync::mpsc::Receiver<Result<MemoryNode>>,
}

impl MemoryStream {
    pub fn new(rx: tokio::sync::mpsc::Receiver<Result<MemoryNode>>) -> Self {
        Self { rx }
    }
}

impl futures_util::Stream for MemoryStream {
    type Item = Result<MemoryNode>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// A stream of memory relationships
pub struct RelationshipStream {
    rx: tokio::sync::mpsc::Receiver<Result<MemoryRelationship>>,
}

impl RelationshipStream {
    pub fn new(rx: tokio::sync::mpsc::Receiver<Result<MemoryRelationship>>) -> Self {
        Self { rx }
    }
}

impl futures_util::Stream for RelationshipStream {
    type Item = Result<MemoryRelationship>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}
