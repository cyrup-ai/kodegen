//! Asynchronous wrapper types for crawling operations.
//!
//! This module provides Future-based wrappers for various crawling operations,
//! allowing them to be spawned and awaited asynchronously.

use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;

use crate::crawl_engine::{CrawlError, CrawlResult};

/// A domain-specific type representing a pending crawl operation.
/// This wraps a oneshot receiver and implements Future so it can be awaited.
pub struct CrawlRequest {
    receiver: oneshot::Receiver<CrawlResult<()>>,
}

impl CrawlRequest {
    /// Create a new `CrawlRequest` from a oneshot receiver
    #[must_use]
    pub fn new(receiver: oneshot::Receiver<CrawlResult<()>>) -> Self {
        Self { receiver }
    }
}

/// Implement Future so users can simply .await the `CrawlRequest`
impl Future for CrawlRequest {
    type Output = CrawlResult<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.receiver).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(CrawlError::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A domain-specific type representing a pending JSON save operation.
/// This wraps a oneshot receiver and implements Future so it can be awaited.
pub struct AsyncJsonSave {
    receiver: oneshot::Receiver<Result<(), anyhow::Error>>,
}

impl AsyncJsonSave {
    /// Create a new `AsyncJsonSave` from a oneshot receiver
    #[must_use]
    pub fn new(receiver: oneshot::Receiver<Result<(), anyhow::Error>>) -> Self {
        Self { receiver }
    }
}

/// Implement Future so users can simply .await the `AsyncJsonSave`
impl Future for AsyncJsonSave {
    type Output = CrawlResult<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.receiver).poll(cx) {
            Poll::Ready(Ok(Ok(()))) => Poll::Ready(Ok(())),
            Poll::Ready(Ok(Err(e))) => Poll::Ready(Err(CrawlError::from(e))),
            Poll::Ready(Err(_)) => Poll::Ready(Err(CrawlError::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A generic browser action wrapper that can be awaited.
/// This is a reusable pattern for all browser-related async operations.
pub struct BrowserAction<T> {
    receiver: oneshot::Receiver<Result<T, anyhow::Error>>,
}

impl<T> BrowserAction<T> {
    /// Create a new `BrowserAction` from a oneshot receiver
    #[must_use]
    pub fn new(receiver: oneshot::Receiver<Result<T, anyhow::Error>>) -> Self {
        Self { receiver }
    }

    /// Create a new `BrowserAction` by spawning an async task
    pub fn spawn<F>(f: F) -> Self
    where
        F: FnOnce() -> Pin<Box<dyn Future<Output = Result<T, anyhow::Error>> + Send>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let result = f().await;
            let _ = tx.send(result);
        });

        Self::new(rx)
    }
}

/// Implement Future so users can simply .await the `BrowserAction`
impl<T> Future for BrowserAction<T>
where
    T: Send,
{
    type Output = CrawlResult<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.receiver).poll(cx) {
            Poll::Ready(Ok(Ok(value))) => Poll::Ready(Ok(value)),
            Poll::Ready(Ok(Err(e))) => Poll::Ready(Err(CrawlError::from(e))),
            Poll::Ready(Err(_)) => Poll::Ready(Err(CrawlError::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}
