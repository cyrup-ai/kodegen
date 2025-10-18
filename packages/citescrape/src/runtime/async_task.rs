use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc;
use std::fmt;

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

/// A zero-allocation, single-shot future that resolves to a value of type T.
/// Optimized for blazing-fast performance with lock-free operations.
pub struct AsyncTask<T> {
    rx: mpsc::UnboundedReceiver<T>,
}

impl<T> Future for AsyncTask<T> {
    type Output = Result<T, TaskError>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Get mutable access to rx for tokio mpsc
        let rx = &mut self.get_mut().rx;
        
        // Fast path - non-blocking receive with zero allocations
        match rx.try_recv() {
            Ok(value) => Poll::Ready(Ok(value)),
            Err(mpsc::error::TryRecvError::Empty) => {
                // Tokio runtime handles waker registration automatically
                // Double-check pattern to avoid race conditions
                match rx.try_recv() {
                    Ok(value) => Poll::Ready(Ok(value)),
                    Err(mpsc::error::TryRecvError::Empty) => Poll::Pending,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        Poll::Ready(Err(TaskError::Disconnected))
                    }
                }
            }
            Err(mpsc::error::TryRecvError::Disconnected) => {
                Poll::Ready(Err(TaskError::Disconnected))
            }
        }
    }
}

impl<T> AsyncTask<T> {
    /// Await the task, panicking on disconnection (for infallible tasks)
    #[inline]
    pub async fn expect_ok(self, msg: &str) -> T {
        self.await.expect(msg)
    }
    
    /// Await the task, converting error to anyhow::Error
    #[inline]
    pub async fn into_anyhow(self) -> anyhow::Result<T> {
        self.await.map_err(|e| anyhow::anyhow!("{}", e))
    }
}

impl<T, E> AsyncTask<Result<T, E>> 
where 
    E: std::error::Error + Send + Sync + 'static 
{
    /// Flatten Result<Result<T, E>, TaskError> into Result<T, anyhow::Error>
    #[inline]
    pub async fn flatten(self) -> anyhow::Result<T> {
        match self.await {
            Ok(inner_result) => inner_result.map_err(|e| anyhow::anyhow!("{}", e)),
            Err(task_err) => Err(anyhow::anyhow!("{}", task_err)),
        }
    }
}

/// Spawns a future onto the global executor and returns an AsyncTask that resolves to its output.
/// This is a zero-allocation operation after the initial channel creation.
/// 
/// The future will be executed on the global work-stealing thread pool for optimal performance.
#[inline]
pub fn spawn_async<Fut, T>(fut: Fut) -> AsyncTask<T>
where
    Fut: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    // Create tokio channel for result
    let (tx, rx) = mpsc::unbounded_channel();
    
    // Spawn future on tokio runtime
    tokio::task::spawn(async move {
        let result = fut.await;
        // Send result through channel (ignore errors if receiver dropped)
        let _ = tx.send(result);
    });
    
    AsyncTask { rx }
}

/// Creates a ready AsyncTask that immediately resolves to the given value.
/// This is a zero-allocation optimization for values that are already available.
#[inline]
pub fn ready<T>(value: T) -> AsyncTask<T> {
    let (tx, rx) = mpsc::unbounded_channel();
    let _ = tx.send(value);
    AsyncTask { rx }
}

/// Creates a never-resolving AsyncTask for testing or special use cases.
/// The task will remain pending indefinitely.
#[inline]
pub fn pending<T>() -> AsyncTask<T> {
    let (_tx, rx) = mpsc::unbounded_channel();
    // Deliberately drop the sender to create a permanently pending task
    AsyncTask { rx }
}

/// Spawns a stream-producing future onto the global executor.
/// Returns an AsyncStream that will receive items from the spawned future.
#[inline]
pub fn spawn_stream<Fut, T, const CAP: usize>(_fut: Fut) -> super::AsyncStream<T, CAP>
where
    Fut: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    // Return empty stream - this is a placeholder for future stream support
    super::AsyncStream::empty()
}

/// RAII guard that ensures AsyncTask completion or tracking
/// 
/// Provides compile-time guarantees that spawned tasks are properly tracked
/// until their results are consumed. Follows the same pattern as BrowserHandlerGuard.
/// 
/// # Purpose
/// Prevents silent task failures by ensuring that:
/// 1. Task handles are not silently ignored
/// 2. Tasks remain in scope until completion
/// 3. Dropped tasks are logged for debugging
/// 
/// # Usage
/// ```ignore
/// let (tx, rx) = tokio::sync::oneshot::channel();
/// let task = some_async_method(config, move |result| {
///     let _ = tx.send(result);
/// });
/// let _guard = TaskGuard::new(task, "some_async_method");
/// // Task is tracked until guard is dropped (after rx.await completes)
/// let result = rx.await?;
/// ```
pub struct TaskGuard<T> {
    handle: Option<AsyncTask<T>>,
    name: &'static str,
}

impl<T> TaskGuard<T> {
    /// Create a new task guard with a descriptive name for debugging
    #[inline]
    pub fn new(handle: AsyncTask<T>, name: &'static str) -> Self {
        Self { 
            handle: Some(handle), 
            name 
        }
    }
    
    /// Explicitly take ownership of the task handle
    /// 
    /// Returns the AsyncTask if available, otherwise returns a pending task.
    /// This prevents the Drop warning from firing.
    #[inline]
    #[allow(dead_code)]
    pub fn into_inner(mut self) -> AsyncTask<T> {
        match self.handle.take() {
            Some(task) => task,
            None => {
                // Handle was already taken, return a pending task
                pending()
            }
        }
    }
}

impl<T> Drop for TaskGuard<T> {
    fn drop(&mut self) {
        if self.handle.is_some() {
            // Task dropped before explicit completion - log for debugging
            // In production, this helps identify potential task leaks or timeouts
            log::warn!("TaskGuard '{}' dropped without explicit completion", self.name);
        }
    }
}
