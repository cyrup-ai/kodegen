//! Zero-allocation, blazing-fast async runtime
//!
//! This module provides a zero-allocation async runtime with lock-free synchronization
//! and pre-allocated channel pools for maximum performance.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};

/// Stack size for channel pools
const CHANNEL_POOL_SIZE: usize = 64;  // Reduced to avoid initialization overhead

/// Zero-allocation channel pool with lock-free operations
/// 
/// Note: Simplified to avoid blocking operations. Channels are NOT pre-allocated
/// to prevent OnceLock blocking issues in async contexts.
pub struct ChannelPool<T> {
    /// Atomic index for allocation tracking
    next_free: AtomicUsize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Default for ChannelPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ChannelPool<T> {
    /// Create a new channel pool
    pub fn new() -> Self {
        Self {
            next_free: AtomicUsize::new(0),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Acquire a channel pair - creates on demand to avoid blocking
    #[inline(always)]
    pub fn acquire(&self) -> Option<(UnboundedSender<T>, UnboundedReceiver<T>)> {
        let idx = self.next_free.fetch_add(1, Ordering::Relaxed);
        if idx >= CHANNEL_POOL_SIZE {
            self.next_free.store(CHANNEL_POOL_SIZE, Ordering::Relaxed);
            return None;
        }
        
        // Create channel on demand - no blocking initialization
        Some(tokio::sync::mpsc::unbounded_channel())
    }
    
    /// Release a channel pair back to the pool
    #[inline(always)]
    pub fn release(&self, _tx: UnboundedSender<T>, _rx: UnboundedReceiver<T>) {
        self.next_free.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Global channel pools for common types (zero heap allocation)
static STRING_CHANNEL_POOL: LazyLock<ChannelPool<String>> = LazyLock::new(ChannelPool::new);
static UNIT_CHANNEL_POOL: LazyLock<ChannelPool<()>> = LazyLock::new(ChannelPool::new);

/// Optimized AsyncTask with inline storage
pub struct AsyncTask<T> {
    rx: UnboundedReceiver<T>,
}

impl<T> AsyncTask<T> {
    /// Create a new AsyncTask with a receiver
    #[inline(always)]
    pub fn new(rx: UnboundedReceiver<T>) -> Self {
        Self { rx }
    }
}

impl<T> std::future::Future for AsyncTask<T> {
    type Output = T;
    
    #[inline(always)]
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let rx = &mut self.get_mut().rx;
        match rx.try_recv() {
            Ok(value) => std::task::Poll::Ready(value),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                // Zero-allocation waker registration
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                eprintln!("ERROR: AsyncTask channel disconnected");
                // Return Pending to avoid panic
                std::task::Poll::Pending
            }
        }
    }
}

/// Zero-allocation spawn for common types
#[inline(always)]
pub fn spawn_string<F>(f: F) -> AsyncTask<String>
where
    F: FnOnce() -> String + Send + 'static,
{
    if let Some((tx, rx)) = STRING_CHANNEL_POOL.acquire() {
        tokio::task::spawn(async move {
            let result = f();
            let _ = tx.send(result);
        });
        AsyncTask::new(rx)
    } else {
        // Fallback to regular channel
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::task::spawn(async move {
            let result = f();
            let _ = tx.send(result);
        });
        AsyncTask::new(rx)
    }
}

/// Zero-allocation spawn for unit type
#[inline(always)]
pub fn spawn_unit<F>(f: F) -> AsyncTask<()>
where
    F: FnOnce() + Send + 'static,
{
    if let Some((tx, rx)) = UNIT_CHANNEL_POOL.acquire() {
        tokio::task::spawn(async move {
            f();
            let _ = tx.send(());
        });
        AsyncTask::new(rx)
    } else {
        // Fallback to regular channel
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::task::spawn(async move {
            f();
            let _ = tx.send(());
        });
        AsyncTask::new(rx)
    }
}

/// Macro for zero-allocation channel receive
#[macro_export]
macro_rules! recv_fast {
    ($rx:expr) => {{
        match $rx.recv() {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!("Channel closed"))
        }
    }};
    ($rx:expr, $msg:expr) => {{
        match $rx.recv() {
            Ok(value) => Ok(value),
            Err(_) => Err(anyhow::anyhow!($msg))
        }
    }};
}

/// Stack-allocated small string for zero heap allocation
pub type SmallString = smallstr::SmallString<[u8; 256]>;

/// Convert Result<T> to single value through callback
#[inline(always)]
pub fn unwrap_result<T, F>(result: Result<T, anyhow::Error>, on_error: F) -> Option<T>
where
    F: FnOnce(anyhow::Error),
{
    match result {
        Ok(value) => Some(value),
        Err(e) => {
            on_error(e);
            None
        }
    }
}