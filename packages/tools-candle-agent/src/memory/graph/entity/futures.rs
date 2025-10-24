//! Future wrappers for entity operations
//!
//! These types wrap oneshot channels to provide Future implementations
//! for entity CRUD operations, enabling async/await syntax.

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;

use super::types::Entity;
use crate::memory::graph::graph_db::{GraphError, Result};

/// Future wrapper for entity creation/update operations
pub struct PendingEntity {
    rx: oneshot::Receiver<Result<Box<dyn Entity>>>,
}

impl PendingEntity {
    pub fn new(rx: oneshot::Receiver<Result<Box<dyn Entity>>>) -> Self {
        Self { rx }
    }
}

impl std::future::Future for PendingEntity {
    type Output = Result<Box<dyn Entity>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => {
                Poll::Ready(Err(GraphError::Other("Channel closed".to_string())))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Future wrapper for entity retrieval operations
pub struct PendingEntityOption {
    rx: oneshot::Receiver<Result<Option<Box<dyn Entity>>>>,
}

impl PendingEntityOption {
    pub fn new(rx: oneshot::Receiver<Result<Option<Box<dyn Entity>>>>) -> Self {
        Self { rx }
    }
}

impl std::future::Future for PendingEntityOption {
    type Output = Result<Option<Box<dyn Entity>>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => {
                Poll::Ready(Err(GraphError::Other("Channel closed".to_string())))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Future wrapper for entity list operations
pub struct PendingEntityList {
    rx: oneshot::Receiver<Result<Vec<Box<dyn Entity>>>>,
}

impl PendingEntityList {
    pub fn new(rx: oneshot::Receiver<Result<Vec<Box<dyn Entity>>>>) -> Self {
        Self { rx }
    }
}

impl std::future::Future for PendingEntityList {
    type Output = Result<Vec<Box<dyn Entity>>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => {
                Poll::Ready(Err(GraphError::Other("Channel closed".to_string())))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Future wrapper for entity count operations
pub struct PendingEntityCount {
    rx: oneshot::Receiver<Result<usize>>,
}

impl PendingEntityCount {
    pub fn new(rx: oneshot::Receiver<Result<usize>>) -> Self {
        Self { rx }
    }
}

impl std::future::Future for PendingEntityCount {
    type Output = Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => {
                Poll::Ready(Err(GraphError::Other("Channel closed".to_string())))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Future wrapper for unit operations (delete, etc.)
pub struct PendingUnit {
    rx: oneshot::Receiver<Result<()>>,
}

impl PendingUnit {
    pub fn new(rx: oneshot::Receiver<Result<()>>) -> Self {
        Self { rx }
    }
}

impl std::future::Future for PendingUnit {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => {
                Poll::Ready(Err(GraphError::Other("Channel closed".to_string())))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
