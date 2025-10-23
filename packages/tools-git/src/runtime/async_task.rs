//! Async task and stream abstractions for `GitGix` operations.
//!
//! Channel-based design for zero-allocation async coordination.

use futures::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::{mpsc, oneshot};

/// Type alias for a pinned, sendable future that returns a Result with a Vec.
type BoxedVecFuture<T, E> = Pin<Box<dyn Future<Output = Result<Vec<T>, E>> + Send>>;

/// Type alias for a factory function that produces a `BoxedVecFuture`.
type FutureFactory<T, E> = Box<dyn FnOnce() -> BoxedVecFuture<T, E> + Send>;

// ============================================================================
// AsyncTask - Single-result async operation
// ============================================================================

/// A handle to an asynchronous task that produces a single result.
///
/// Uses oneshot channel internally for efficient one-time communication.
pub struct AsyncTask<T> {
    rx: oneshot::Receiver<T>,
}

impl<T> AsyncTask<T>
where
    T: Send + 'static,
{
    /// Create from oneshot receiver (for advanced use).
    #[inline]
    #[must_use]
    pub fn new(rx: oneshot::Receiver<T>) -> Self {
        Self { rx }
    }

    /// Spawn a blocking operation on a background thread.
    ///
    /// Maintains API compatibility with existing code while using
    /// channel-based coordination internally.
    #[inline]
    pub fn spawn<F>(f: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        tokio::task::spawn_blocking(move || {
            let _ = tx.send(f());
        });
        Self::new(rx)
    }

    /// Spawn an async operation.
    ///
    /// For operations that are already async and don't need `spawn_blocking`.
    #[inline]
    pub fn spawn_async<F>(future: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        tokio::task::spawn(async move {
            let _ = tx.send(future.await);
        });
        Self::new(rx)
    }
}

impl<T> Future for AsyncTask<T> {
    type Output = Result<T, oneshot::error::RecvError>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.rx).poll(cx)
    }
}

// ============================================================================
// AsyncStream - Multi-result streaming operation
// ============================================================================

/// A handle to an asynchronous stream that produces multiple results.
///
/// Uses unbounded mpsc channel for true streaming without memory accumulation.
pub struct AsyncStream<T> {
    rx: mpsc::UnboundedReceiver<T>,
}

impl<T> AsyncStream<T> {
    /// Create from unbounded receiver.
    #[inline]
    #[must_use]
    pub fn new(rx: mpsc::UnboundedReceiver<T>) -> Self {
        Self { rx }
    }

    /// Create from a vector (for testing/simple cases).
    ///
    /// Internally spawns a task to send items through channel.
    #[must_use]
    pub fn from_vec(items: Vec<T>) -> Self
    where
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::task::spawn(async move {
            for item in items {
                if tx.send(item).is_err() {
                    break; // Receiver dropped
                }
            }
        });
        Self::new(rx)
    }
}

impl<T> Stream for AsyncStream<T> {
    type Item = T;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

// ============================================================================
// EmitterBuilder - Batch-to-stream conversion for API results
// ============================================================================

/// Builder for converting batch API results into streams.
///
/// Used by GitHub API operations that fetch all results into a Vec
/// and then stream them one at a time to the caller.
pub struct EmitterBuilder<T, E> {
    future_factory: FutureFactory<T, E>,
}

impl<T, E> EmitterBuilder<T, E>
where
    T: Send + 'static,
    E: Send + 'static,
{
    /// Create a new emitter builder with a future factory.
    ///
    /// The factory returns a pinned future that produces Result<Vec<T>, E>.
    #[must_use]
    pub fn new(future_factory: FutureFactory<T, E>) -> Self {
        Self { future_factory }
    }

    /// Emit items from the future result through a stream.
    ///
    /// - `transform`: Function to transform each item (typically identity |v| v)
    /// - `on_error`: Error handler called if the future fails (typically no-op |_| {})
    pub fn emit<F, G>(self, transform: F, on_error: G) -> AsyncStream<Result<T, E>>
    where
        F: Fn(T) -> T + Send + 'static,
        G: Fn(&E) + Send + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let future = (self.future_factory)();
            match future.await {
                Ok(items) => {
                    for item in items {
                        let transformed = transform(item);
                        if tx.send(Ok(transformed)).is_err() {
                            break; // Receiver dropped
                        }
                    }
                }
                Err(e) => {
                    on_error(&e);
                    let _ = tx.send(Err(e));
                }
            }
        });

        AsyncStream::new(rx)
    }
}
