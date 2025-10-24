//! Candle Async Stream - Pure tokio streaming utilities
//!
//! Provides helper functions for creating and working with tokio streams.
//! 100% tokio async - no sync/async bridging.

use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

// Re-export commonly used tokio_stream types
pub use tokio_stream::wrappers::ReceiverStream;
pub use tokio_stream::{Stream, StreamExt};

/// Create a stream from a spawned async task
///
/// # Example
/// ```rust
/// let stream = spawn_stream(|tx| async move {
///     for i in 0..10 {
///         let _ = tx.send(i).await;
///     }
/// });
/// ```
pub fn spawn_stream<T, F, Fut>(f: F) -> impl Stream<Item = T>
where
    T: Send + 'static,
    F: FnOnce(mpsc::UnboundedSender<T>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(f(tx));
    UnboundedReceiverStream::new(rx)
}

/// Create a stream from an iterator
pub fn from_iter<T, I>(iter: I) -> impl Stream<Item = T>
where
    I: IntoIterator<Item = T>,
{
    tokio_stream::iter(iter)
}

/// Create a stream from a single value
pub fn once<T>(value: T) -> impl Stream<Item = T> {
    tokio_stream::once(value)
}

/// Create an empty stream
pub fn empty<T>() -> impl Stream<Item = T> {
    tokio_stream::empty()
}

/// Type alias for backward compatibility during migration
pub type CandleStream<T> = Box<dyn Stream<Item = T> + Send + Unpin>;
