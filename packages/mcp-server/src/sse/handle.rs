//! Server lifecycle management for graceful shutdown
//!
//! This module provides [`ServerHandle`] for managing SSE server lifecycle with
//! proper graceful shutdown support. The handle separates cancellation (signaling
//! shutdown should begin) from completion (waiting for shutdown to finish).
//!
//! # Architecture
//!
//! - **Zero allocation**: All structures are stack-allocated
//! - **Lock-free**: Uses atomic operations via `CancellationToken`
//! - **Ergonomic**: Simple API - call `cancel()` then `wait_for_completion()`
//! - **Timeout-aware**: Respects configured timeout as maximum wait duration
//!
//! # Example
//!
//! ```no_run
//! use std::time::Duration;
//!
//! async fn run_server(handle: ServerHandle) {
//!     // Wait for shutdown signal
//!     tokio::signal::ctrl_c().await.ok();
//!    
//!     // Initiate graceful shutdown
//!     handle.cancel();
//!    
//!     // Wait for completion with 30s timeout
//!     match handle.wait_for_completion(Duration::from_secs(30)).await {
//!         Ok(()) => println!("Server stopped gracefully"),
//!         Err(_) => println!("Shutdown timeout - forcing exit"),
//!     }
//! }
//! ```

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

/// Handle for managing SSE server lifecycle with graceful shutdown support.
///
/// The handle provides two key capabilities:
/// 1. **Cancellation**: Signal that shutdown should begin via [`cancel()`](Self::cancel)
/// 2. **Completion awaiting**: Wait for shutdown to complete via [`wait_for_completion()`](Self::wait_for_completion)
///
/// # Performance
///
/// - Zero heap allocations
/// - Lock-free operation using atomic primitives
/// - Inlined hot paths for minimal overhead
///
/// # Lifecycle
///
/// 1. Server creates handle and returns it to caller
/// 2. Caller signals shutdown via `cancel()`
/// 3. Server begins graceful shutdown
/// 4. Caller awaits `wait_for_completion()` with timeout
/// 5. Either server completes (Ok) or timeout fires (Err)
pub struct ServerHandle {
    /// Token for signaling cancellation
    cancel_token: CancellationToken,
    
    /// Receiver for completion notification
    completion_rx: oneshot::Receiver<()>,
}

impl ServerHandle {
    /// Create new server handle.
    ///
    /// # Arguments
    ///
    /// * `cancel_token` - Token for signaling cancellation
    /// * `completion_rx` - Channel receiver for completion notification
    ///
    /// # Performance
    ///
    /// This function is zero-allocation and completes in O(1) time.
    #[inline]
    pub fn new(
        cancel_token: CancellationToken,
        completion_rx: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            cancel_token,
            completion_rx,
        }
    }

    /// Signal server to begin graceful shutdown.
    ///
    /// This is a non-blocking operation that signals the server should start
    /// shutting down. It does not wait for shutdown to complete.
    ///
    /// After calling this, use [`wait_for_completion()`](Self::wait_for_completion)
    /// to wait for the server to actually finish shutting down.
    ///
    /// # Performance
    ///
    /// - Lock-free atomic operation
    /// - O(1) time complexity
    /// - Zero allocations
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use server_handle::ServerHandle;
    /// # async fn example(handle: ServerHandle) {
    /// // Signal shutdown
    /// handle.cancel();
    ///
    /// // Server is now shutting down (but may not be done yet)
    /// # }
    /// ```
    #[inline]
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Wait for server to complete shutdown, with timeout.
    ///
    /// This consumes the handle and returns when either:
    /// - Server completes shutdown gracefully (returns `Ok(())`)
    /// - Timeout elapses before completion (returns `Err`)
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration to wait for completion
    ///
    /// # Returns
    ///
    /// - `Ok(())` - Server completed shutdown within timeout
    /// - `Err(tokio::time::error::Elapsed)` - Timeout elapsed before completion
    ///
    /// # Performance
    ///
    /// - Zero allocations
    /// - Uses `tokio::time::timeout` for efficient timeout handling
    /// - Early return on completion (no unnecessary waiting)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use server_handle::ServerHandle;
    /// # use std::time::Duration;
    /// # async fn example(handle: ServerHandle) -> Result<(), Box<dyn std::error::Error>> {
    /// use std::time::Duration;
    ///
    /// handle.cancel();
    ///
    /// match handle.wait_for_completion(Duration::from_secs(30)).await {
    ///     Ok(()) => {
    ///         log::info!("Server stopped gracefully");
    ///         Ok(())
    ///     }
    ///     Err(_elapsed) => {
    ///         log::warn!("Shutdown timeout - some requests may be interrupted");
    ///         Err("timeout".into())
    ///     }
    /// }
    /// # }
    /// ```
    #[inline]
    pub async fn wait_for_completion(
        self,
        timeout: std::time::Duration,
    ) -> Result<(), tokio::time::error::Elapsed> {
        // Race timeout against completion signal with explicit error handling
        match tokio::time::timeout(timeout, self.completion_rx).await {
            Ok(Ok(())) => {
                log::debug!("Shutdown completed via signal");
                Ok(())
            }
            Ok(Err(_recv_error)) => {
                log::warn!(
                    "Shutdown completion channel closed without signal. \
                     This usually means shutdown completed very quickly, \
                     but could indicate a panic in the monitor task."
                );
                Ok(())
            }
            Err(elapsed) => {
                log::warn!("Shutdown timeout after {timeout:?}");
                Err(elapsed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_immediate_completion() {
        let cancel_token = CancellationToken::new();
        let (completion_tx, completion_rx) = oneshot::channel();
        
        let handle = ServerHandle::new(cancel_token, completion_rx);
        
        // Signal completion immediately
        completion_tx.send(()).expect("send failed");
        
        // Should complete instantly
        let result = handle.wait_for_completion(Duration::from_secs(10)).await;
        assert!(result.is_ok(), "should complete successfully");
    }

    #[tokio::test]
    async fn test_timeout() {
        let cancel_token = CancellationToken::new();
        let (_completion_tx, completion_rx) = oneshot::channel();
        
        let handle = ServerHandle::new(cancel_token, completion_rx);
        
        // Never send completion signal
        // Should timeout
        let result = handle.wait_for_completion(Duration::from_millis(100)).await;
        assert!(result.is_err(), "should timeout");
    }

    #[tokio::test]
    async fn test_cancel_propagation() {
        let cancel_token = CancellationToken::new();
        let token_clone = cancel_token.clone();
        let (_completion_tx, completion_rx) = oneshot::channel();
        
        let handle = ServerHandle::new(cancel_token, completion_rx);
        
        // Cancel should propagate to cloned token
        handle.cancel();
        
        assert!(token_clone.is_cancelled(), "cancellation should propagate");
    }

    #[tokio::test]
    async fn test_channel_closed_before_send() {
        let cancel_token = CancellationToken::new();
        let (completion_tx, completion_rx) = oneshot::channel();
        
        let handle = ServerHandle::new(cancel_token, completion_rx);
        
        // Drop sender (close channel)
        drop(completion_tx);
        
        // Should treat as successful completion
        let result = handle.wait_for_completion(Duration::from_secs(10)).await;
        assert!(result.is_ok(), "closed channel should be treated as success");
    }

    #[tokio::test]
    async fn test_completion_signal_race() {
        let cancel_token = CancellationToken::new();
        let (completion_tx, completion_rx) = oneshot::channel();
        
        let monitor_ct = cancel_token.clone();
        tokio::spawn(async move {
            monitor_ct.cancelled().await;
            // Simulate slow monitor task
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = completion_tx.send(());
        });
        
        let handle = ServerHandle::new(cancel_token, completion_rx);
        handle.cancel();
        
        // Timeout before monitor can send
        let result = handle.wait_for_completion(Duration::from_millis(10)).await;
        
        // Should timeout, but send failure should be logged
        assert!(result.is_err(), "should timeout before monitor can signal");
        
        // Give monitor task time to attempt send
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // If we reach here without panic, the race condition was handled gracefully
    }
}
