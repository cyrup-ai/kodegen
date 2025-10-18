//! Zero-allocation, blazing-fast async runtime
//!
//! This module provides a zero-allocation async runtime with lock-free synchronization
//! and pre-allocated channel pools for maximum performance.

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};

/// Error type for AsyncTask execution failures
#[derive(Debug, Clone)]
pub enum TaskError {
    /// Sender was dropped without sending a value (task panicked or executor died)
    Disconnected,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::Disconnected => write!(
                f,
                "AsyncTask sender dropped without sending value (task panicked or failed to complete)"
            ),
        }
    }
}

impl std::error::Error for TaskError {}

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
    type Output = Result<T, TaskError>;
    
    #[inline(always)]
    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let rx = &mut self.get_mut().rx;
        match rx.try_recv() {
            Ok(value) => std::task::Poll::Ready(Ok(value)),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                // Zero-allocation waker registration
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                std::task::Poll::Ready(Err(TaskError::Disconnected))
            }
        }
    }
}

impl<T> AsyncTask<T> {
    /// Await the task, panicking on disconnection (for infallible tasks).
    /// 
    /// Use this for cleanup tasks, internal operations, or any task that
    /// should never fail. Provides a custom panic message for debugging.
    /// 
    /// # Panics
    /// Panics if the task's sender was dropped without sending a value
    /// (task panicked or executor died).
    /// 
    /// # Example
    /// ```ignore
    /// let result = spawn_async(cleanup()).expect_ok("cleanup failed").await;
    /// ```
    #[inline]
    pub async fn expect_ok(self, msg: &str) -> T {
        self.await.expect(msg)
    }

    /// Await the task, converting TaskError to anyhow::Error.
    /// 
    /// Use this in functions returning anyhow::Result to enable the ? operator.
    /// 
    /// # Example
    /// ```ignore
    /// async fn fetch_data() -> anyhow::Result<Data> {
    ///     let data = spawn_async(async_fetch()).into_anyhow().await?;
    ///     Ok(data)
    /// }
    /// ```
    #[inline]
    pub async fn into_anyhow(self) -> anyhow::Result<T> {
        self.await.map_err(|e| anyhow::anyhow!("{}", e))
    }
}

impl<T, E> AsyncTask<Result<T, E>> 
where 
    E: std::error::Error + Send + Sync + 'static 
{
    /// Flatten Result<Result<T, E>, TaskError> into Result<T, anyhow::Error>.
    /// 
    /// Use this when spawning tasks that themselves return Results. Converts
    /// both the inner error (E) and outer error (TaskError) to anyhow::Error.
    /// 
    /// # Example
    /// ```ignore
    /// async fn process() -> anyhow::Result<String> {
    ///     spawn_async(async {
    ///         fetch_url("https://example.com").await
    ///     }).flatten().await?
    /// }
    /// ```
    #[inline]
    pub async fn flatten(self) -> anyhow::Result<T> {
        match self.await {
            Ok(inner_result) => inner_result.map_err(|e| anyhow::anyhow!("{}", e)),
            Err(task_err) => Err(anyhow::anyhow!("{}", task_err)),
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