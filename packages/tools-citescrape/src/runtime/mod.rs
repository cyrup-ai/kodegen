//! Zero-allocation, blazing-fast async runtime
//!
//! This module provides lock-free, zero-allocation async primitives optimized for
//! maximum performance with elegant ergonomic APIs.

pub mod async_stream;
pub mod async_wrappers;
pub mod channel;

pub use async_stream::{AsyncStream, StreamSender, TrySendError};
pub use async_wrappers::{AsyncJsonSave, BrowserAction, CrawlRequest};
pub use channel::*;

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
        $crate::recv_async!($rx, "Channel closed unexpectedly", 30)
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

/// Create channel with optimal configuration
#[inline(always)]
#[must_use]
pub fn create_channel<T>() -> (
    tokio::sync::mpsc::UnboundedSender<T>,
    tokio::sync::mpsc::UnboundedReceiver<T>,
) {
    tokio::sync::mpsc::unbounded_channel()
}
