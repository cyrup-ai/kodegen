//! Zero-allocation, blazing-fast async runtime
//!
//! This module provides lock-free, zero-allocation async primitives optimized for
//! maximum performance with elegant ergonomic APIs.

pub mod async_stream;
pub mod async_task;
pub mod async_wrappers;
pub mod channel;
pub mod thread_pool;
pub mod zero_alloc;

pub use async_stream::{AsyncStream, StreamSender, TrySendError};
pub use async_task::{spawn_async, spawn_stream, AsyncTask, TaskGuard, ready, pending, TaskError};
pub use async_wrappers::{AsyncJsonSave, BrowserAction, CrawlRequest};
pub use channel::*;
pub use thread_pool::ThreadPool;
pub use zero_alloc::{spawn_string, spawn_unit, SmallString, unwrap_result};

// DEPRECATED: recv_async! macro - blocks async runtime threads!
// 
// This macro uses blocking recv_timeout() which defeats the purpose of async/await.
// All production code has been migrated to tokio::sync::oneshot with .await patterns.
// 
// DO NOT USE THIS MACRO IN NEW CODE!
// 
// Migration pattern:
// ```ignore
// // OLD - BLOCKING (DO NOT USE)
// let (tx, rx) = std::sync::mpsc::channel();
// recv_async!(rx, "error message")?;
// 
// // NEW - ASYNC (USE THIS)
// let (tx, rx) = tokio::sync::oneshot::channel();
// rx.await.map_err(|_| anyhow::anyhow!("error message"))?;
// ```
//
// This macro is commented out to prevent new usage. If you need it for legacy code,
// uncomment it, but plan to migrate away from it as soon as possible.
/*
#[macro_export]
macro_rules! recv_async {
    ($rx:expr) => {{
        $crate::recv_async!($rx, "AsyncTask channel closed unexpectedly", 30)
    }};
    ($rx:expr, $msg:expr) => {{
        $crate::recv_async!($rx, $msg, 30)
    }};
    ($rx:expr, $msg:expr, $timeout_secs:expr) => {{
        use std::time::Duration;
        match $rx.recv_timeout(Duration::from_secs($timeout_secs)) {
            Ok(value) => Ok(value),
            Err(e) => {
                // Handle both std::sync::mpsc and crossbeam_channel error types
                let error_msg = format!("{:?}", e);
                if error_msg.contains("Timeout") {
                    Err(anyhow::anyhow!("{} (timeout after {}s)", $msg, $timeout_secs))
                } else {
                    Err(anyhow::anyhow!("{} (task panicked or channel closed)", $msg))
                }
            }
        }
    }};
}
*/

/// Pattern for handling Result in callbacks - unwrap and send value
/// 
/// Usage:
/// ```ignore
/// let (tx, rx) = std::sync::mpsc::channel();
/// let _task = some_async_method(param, move |result| {
///     on_result!(result, tx, "Error message");
/// });
/// let value = recv_async!(rx)?;
/// ```
#[macro_export]
macro_rules! on_result {
    ($result:expr, $tx:expr, $err_msg:expr) => {
        match $result {
            Ok(value) => { let _ = $tx.send(value); }
            Err(e) => { 
                log::error!("{}: {}", $err_msg, e);
                // For critical errors, you might want to send a default value
                // or handle differently based on your needs
            }
        }
    };
}

/// Optimized callback pattern for unit results
#[macro_export]
macro_rules! on_unit_result {
    ($result:expr, $tx:expr) => {
        match $result {
            Ok(()) => { let _ = $tx.send(()); }
            Err(e) => { log::error!("Operation failed: {}", e); }
        }
    };
}

/// Global executor module providing zero-allocation async runtime services.
pub mod executor {
    use super::thread_pool::ThreadPool;
    use futures_executor::block_on;
    use std::{
        future::Future,
        sync::{LazyLock, atomic::AtomicBool},
        task::Waker,
    };

    /// Global executor state with lock-free coordination
    struct GlobalExecutor {
        /// Work-stealing thread pool for job execution
        pool: ThreadPool,
        /// Atomic flag for executor state
        #[allow(dead_code)]
        running: AtomicBool,
    }

    /// Global executor instance initialized on first access.
    static GLOBAL_EXECUTOR: LazyLock<GlobalExecutor> = LazyLock::new(|| {
        GlobalExecutor {
            pool: ThreadPool::new(),
            running: AtomicBool::new(true),
        }
    });

    /// Spawn a future onto the global executor (zero allocation for small futures)
    #[inline(always)]
    pub fn spawn<F>(future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        GLOBAL_EXECUTOR.pool.execute(move || {
            block_on(future);
        });
    }
    
    /// Register a waker for notifications (no-op in this simple implementation)
    #[inline(always)]
    pub fn register_waker(_waker: Waker) {
        // In a more sophisticated executor, this would register the waker
        // for notifications when work is available. For our simple executor,
        // we rely on busy polling which is acceptable for this use case.
    }
}

/// Create channel with optimal configuration
#[inline(always)]
pub fn create_channel<T>() -> (tokio::sync::mpsc::UnboundedSender<T>, tokio::sync::mpsc::UnboundedReceiver<T>) {
    tokio::sync::mpsc::unbounded_channel()
}