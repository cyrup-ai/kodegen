use std::time::Duration;
use tokio::sync::mpsc::{
    Receiver, // Bounded channel types
    Sender,
    UnboundedReceiver,
    UnboundedSender,
    error::{SendError, TryRecvError, TrySendError},
};

/// Internal enum unifying bounded and unbounded sender types
enum TxInner<T> {
    Bounded(Sender<T>),
    Unbounded(UnboundedSender<T>),
}

/// Internal enum unifying bounded and unbounded receiver types
enum RxInner<T> {
    Bounded(Receiver<T>, usize),
    Unbounded(UnboundedReceiver<T>),
}

impl<T> Clone for TxInner<T> {
    fn clone(&self) -> Self {
        match self {
            TxInner::Bounded(s) => TxInner::Bounded(s.clone()),
            TxInner::Unbounded(s) => TxInner::Unbounded(s.clone()),
        }
    }
}

/// Zero-allocation sender wrapper optimized for high-throughput message passing.
/// All operations are lock-free and designed for maximum performance.
pub struct Tx<T>(TxInner<T>);

impl<T: Send + 'static> Tx<T> {
    /// Sends a message asynchronously.
    /// Bounded channels may wait for capacity, unbounded never wait.
    #[inline]
    pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
        match &self.0 {
            TxInner::Bounded(s) => s.send(value).await.map_err(|e| SendError(e.0)),
            TxInner::Unbounded(s) => s.send(value).map_err(|e| SendError(e.0)),
        }
    }

    /// Attempts to send a message immediately without blocking.
    /// Returns error if channel is full or disconnected.
    #[inline]
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        match &self.0 {
            TxInner::Bounded(s) => s.try_send(value),
            TxInner::Unbounded(s) => s.send(value).map_err(|e| TrySendError::Closed(e.0)),
        }
    }

    /// Returns true if the channel is closed and all receivers have been dropped.
    #[inline]
    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        match &self.0 {
            TxInner::Bounded(s) => s.is_closed(),
            TxInner::Unbounded(s) => s.is_closed(),
        }
    }

    /// Returns the number of messages currently in the channel.
    /// Returns None for unbounded channels (tokio doesn't expose len for unbounded).
    #[inline]
    #[must_use]
    pub fn len(&self) -> Option<usize> {
        match &self.0 {
            TxInner::Bounded(s) => {
                // Accurate count: total capacity - remaining capacity
                Some(s.max_capacity() - s.capacity())
            }
            TxInner::Unbounded(_) => {
                // Tokio doesn't expose len() for UnboundedSender
                None
            }
        }
    }

    /// Returns true if the channel is empty.
    /// Only for bounded channels where we can check len.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == Some(0)
    }

    /// Returns the channel capacity, None for unbounded channels.
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> Option<usize> {
        match &self.0 {
            TxInner::Bounded(s) => Some(s.max_capacity()),
            TxInner::Unbounded(_) => None,
        }
    }
}

impl<T> Clone for Tx<T> {
    #[inline]
    fn clone(&self) -> Self {
        Tx(self.0.clone())
    }
}

/// Zero-allocation receiver wrapper optimized for high-throughput message consumption.
/// All operations are lock-free and designed for maximum performance.
pub struct Rx<T>(RxInner<T>);

impl<T: Send + 'static> Rx<T> {
    /// Receives a message asynchronously.
    /// Returns None when channel is closed and empty.
    #[inline]
    pub async fn recv(&mut self) -> Option<T> {
        match &mut self.0 {
            RxInner::Bounded(r, _) => r.recv().await,
            RxInner::Unbounded(r) => r.recv().await,
        }
    }

    /// Attempts to receive a message immediately without blocking.
    /// Returns error if channel is empty or disconnected.
    #[inline]
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        match &mut self.0 {
            RxInner::Bounded(r, _) => r.try_recv(),
            RxInner::Unbounded(r) => r.try_recv(),
        }
    }

    /// Receives a message with a timeout.
    /// Returns Err if timeout expires or channel is closed.
    #[inline]
    pub async fn recv_timeout(&mut self, duration: Duration) -> Result<T, ()> {
        match tokio::time::timeout(duration, self.recv()).await {
            Ok(Some(val)) => Ok(val),
            _ => Err(()),
        }
    }

    /// Returns true if the channel is empty.
    /// Note: This method requires &mut self as it uses `try_recv` to check state.
    #[inline]
    pub fn is_empty(&mut self) -> bool {
        match &mut self.0 {
            RxInner::Bounded(r, _) => {
                // Check if value is immediately available
                matches!(
                    r.try_recv(),
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty)
                )
            }
            RxInner::Unbounded(r) => {
                // Conservative: check actual state
                matches!(
                    r.try_recv(),
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty)
                )
            }
        }
    }

    /// Returns the channel capacity, None for unbounded channels.
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> Option<usize> {
        match &self.0 {
            RxInner::Bounded(_, cap) => Some(*cap),
            RxInner::Unbounded(_) => None,
        }
    }
}

/// Creates an unbounded channel optimized for maximum throughput.
/// The channel can hold an unlimited number of messages (subject to available memory).
///
/// Returns (sender, receiver) pair for lock-free communication.
#[inline]
#[must_use]
pub fn unbounded<T>() -> (Tx<T>, Rx<T>) {
    let (s, r) = tokio::sync::mpsc::unbounded_channel();
    (Tx(TxInner::Unbounded(s)), Rx(RxInner::Unbounded(r)))
}

/// Creates a bounded channel with the specified capacity.
///
/// Returns (sender, receiver) pair for lock-free communication with flow control.
#[inline]
#[must_use]
pub fn bounded<T>(cap: usize) -> (Tx<T>, Rx<T>) {
    let (s, r) = tokio::sync::mpsc::channel(cap);
    (Tx(TxInner::Bounded(s)), Rx(RxInner::Bounded(r, cap)))
}

/// Performance-optimized channel creation for single-shot communication.
/// Equivalent to bounded(1) but may have optimizations for the single-item use case.
#[inline]
#[must_use]
pub fn oneshot<T>() -> (Tx<T>, Rx<T>) {
    bounded(1)
}

/// Creates a channel pair optimized for high-frequency, low-latency communication.
/// Uses a small bounded capacity to maintain cache locality while preventing blocking.
#[inline]
#[must_use]
pub fn sync_channel<T>(cap: usize) -> (Tx<T>, Rx<T>) {
    bounded(cap.max(1)) // Ensure at least capacity of 1
}

impl<T: Send + 'static> std::fmt::Debug for Tx<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Tx");
        if let Some(len) = self.len() {
            debug.field("len", &len);
        }
        debug
            .field("capacity", &self.capacity())
            .field("is_empty", &self.is_empty())
            .finish()
    }
}

impl<T: Send + 'static> std::fmt::Debug for Rx<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rx")
            .field("capacity", &self.capacity())
            // Note: is_empty and len omitted - is_empty requires &mut self, len not exposed by tokio
            .finish()
    }
}
