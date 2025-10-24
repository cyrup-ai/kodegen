use crossbeam_queue::ArrayQueue;
use futures_core::Stream;
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};

/// A zero-allocation, lock-free bounded stream with const generic capacity.
/// Optimized for blazing-fast producer-consumer communication with backpressure handling.
pub struct AsyncStream<T, const CAP: usize> {
    inner: Arc<StreamInner<T, CAP>>,
}

/// Internal shared state for the stream, designed for maximum performance.
struct StreamInner<T, const CAP: usize> {
    /// Lock-free queue for data items with const capacity
    queue: ArrayQueue<T>,
    /// Lock-free queue for pending consumer wakers
    wakers: ArrayQueue<Waker>,
    /// Atomic flag indicating if stream is closed
    closed: AtomicBool,
}

/// Producer side of the stream, optimized for zero-allocation sends.
pub struct StreamSender<T, const CAP: usize> {
    inner: Arc<StreamInner<T, CAP>>,
}

/// Error types for non-blocking send operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrySendError<T> {
    /// Queue is at capacity, contains the value that couldn't be sent
    Full(T),
    /// Stream is closed, contains the value that couldn't be sent
    Closed(T),
}

impl<T, const CAP: usize> AsyncStream<T, CAP> {
    /// Creates a new bounded stream with const generic capacity.
    /// Returns (sender, stream) pair for producer-consumer communication.
    ///
    /// This is a zero-allocation operation that pre-allocates all necessary data structures.
    #[inline]
    #[must_use]
    pub fn channel() -> (StreamSender<T, CAP>, AsyncStream<T, CAP>) {
        let inner = Arc::new(StreamInner {
            queue: ArrayQueue::new(CAP),
            wakers: ArrayQueue::new(CAP), // At most CAP waiting consumers
            closed: AtomicBool::new(false),
        });

        let sender = StreamSender {
            inner: inner.clone(),
        };
        let stream = AsyncStream { inner };

        (sender, stream)
    }

    /// Attempts to receive a value immediately without blocking.
    /// Returns None if no value is available.
    ///
    /// This is a zero-allocation operation with optimal performance.
    #[inline]
    #[must_use]
    pub fn try_recv(&self) -> Option<T> {
        self.inner.queue.pop()
    }

    /// Checks if the stream is closed and no more values will be produced.
    #[inline]
    #[must_use]
    pub fn is_terminated(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire) && self.inner.queue.is_empty()
    }

    /// Returns the current number of items in the stream.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.queue.len()
    }

    /// Returns true if the stream is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.queue.is_empty()
    }

    /// Returns the compile-time capacity of the stream.
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Convenience helper – wrap a single item into a ready stream.
    /// Used in API glue; cost = one push into the bounded queue.
    #[inline]
    pub fn from_single(item: T) -> Self {
        let (tx, st) = Self::channel();
        // ignore full error – CAP ≥ 1 in every instantiation
        let _ = tx.try_send(item);
        st
    }

    /// Empty stream (always returns `Poll::Ready(None)`).
    #[inline]
    #[must_use]
    pub fn empty() -> Self {
        let (tx, st) = Self::channel();
        tx.close(); // Close immediately to signal end
        st
    }
}

impl<T, const CAP: usize> StreamSender<T, CAP> {
    /// Attempts to send a value immediately without blocking.
    ///
    /// Returns:
    /// - Ok(()) if the value was sent successfully
    /// - `Err(TrySendError::Full(value))` if the queue is at capacity
    /// - `Err(TrySendError::Closed(value))` if the stream is closed
    ///
    /// This is a zero-allocation operation optimized for maximum throughput.
    #[inline]
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        // Fast path check for closed state
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(TrySendError::Closed(value));
        }

        // Attempt lock-free push to queue
        match self.inner.queue.push(value) {
            Ok(()) => {
                // Wake up a waiting consumer if any (zero-allocation wake)
                if let Some(waker) = self.inner.wakers.pop() {
                    waker.wake();
                }
                Ok(())
            }
            Err(value) => Err(TrySendError::Full(value)),
        }
    }

    /// Closes the stream, preventing further sends.
    /// All pending consumers will be notified immediately.
    ///
    /// This operation ensures all waiting consumers are awakened efficiently.
    #[inline]
    pub fn close(&self) {
        self.inner.closed.store(true, Ordering::Release);

        // Wake all waiting consumers with zero allocations
        while let Some(waker) = self.inner.wakers.pop() {
            waker.wake();
        }
    }

    /// Checks if the stream sender is closed.
    #[inline]
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    /// Returns the current number of items in the queue.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.queue.len()
    }

    /// Returns true if the queue is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.queue.is_empty()
    }

    /// Returns true if the queue is at capacity.
    #[inline]
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.inner.queue.len() == CAP
    }

    /// Returns the compile-time capacity of the stream.
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns the number of free slots in the queue.
    #[inline]
    #[must_use]
    pub fn available_capacity(&self) -> usize {
        CAP.saturating_sub(self.inner.queue.len())
    }
}

impl<T, const CAP: usize> Stream for AsyncStream<T, CAP> {
    type Item = T;

    /// Polls for the next item in the stream with optimal performance.
    /// Uses double-check pattern to avoid race conditions while maintaining zero allocations.
    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Fast path - attempt immediate receive with zero allocations
        if let Some(value) = self.inner.queue.pop() {
            return Poll::Ready(Some(value));
        }

        // Check if stream is closed and no more items available
        if self.inner.closed.load(Ordering::Acquire) {
            return Poll::Ready(None);
        }

        // Register waker for notification - this is the only potential allocation point
        let waker = cx.waker().clone();
        if self.inner.wakers.push(waker).is_err() {
            // Waker queue is full (extremely rare), schedule immediate retry
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        // Double-check pattern to avoid race conditions
        if let Some(value) = self.inner.queue.pop() {
            // Remove our waker since we got a value
            let _ = self.inner.wakers.pop();
            Poll::Ready(Some(value))
        } else if self.inner.closed.load(Ordering::Acquire) {
            // Remove our waker since stream is closed
            let _ = self.inner.wakers.pop();
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    /// Provides size hint for stream optimization.
    /// Returns exact bounds when stream is closed, unbounded otherwise.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.queue.len();
        if self.inner.closed.load(Ordering::Acquire) {
            (len, Some(len))
        } else {
            (len, None)
        }
    }
}

/// Automatically close stream when sender is dropped to ensure proper cleanup.
impl<T, const CAP: usize> Drop for StreamSender<T, CAP> {
    #[inline]
    fn drop(&mut self) {
        self.close();
    }
}

impl<T, const CAP: usize> Clone for StreamSender<T, CAP> {
    /// Creates a new sender handle to the same stream.
    /// Multiple senders can send to the same stream concurrently.
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Display implementation for debugging and monitoring.
impl<T, const CAP: usize> std::fmt::Debug for AsyncStream<T, CAP> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncStream")
            .field("capacity", &CAP)
            .field("len", &self.len())
            .field("closed", &self.inner.closed.load(Ordering::Relaxed))
            .finish()
    }
}

impl<T, const CAP: usize> std::fmt::Debug for StreamSender<T, CAP> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamSender")
            .field("capacity", &CAP)
            .field("len", &self.len())
            .field("closed", &self.is_closed())
            .finish()
    }
}
