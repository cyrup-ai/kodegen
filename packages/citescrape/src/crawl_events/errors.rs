//! Error types for event bus operations
//!
//! This module defines the various error conditions that can occur
//! during event bus operations.

/// Error types for event bus operations
#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    /// Failed to publish one or more events in a batch
    #[error("Failed to publish event: {0}")]
    PublishFailed(String),

    /// No active subscribers when publishing
    #[error("No active subscribers")]
    NoSubscribers,

    /// Receiver couldn't keep up, missed messages
    #[error("Receiver lagged behind, missed {0} messages")]
    ReceiverLagged(u64),

    /// Event bus or receiver was closed
    #[error("Event bus shutdown")]
    Shutdown,

    /// Channel is at capacity and backpressure mode is Error
    #[error("Event channel is full (capacity exceeded)")]
    ChannelFull,

    /// Drain timeout during shutdown - some operations still pending
    #[error("Drain timeout: {pending_operations} operations still pending")]
    DrainTimeout { pending_operations: usize },

    /// Publish operation timed out waiting for channel space
    #[error("Publish timed out waiting for channel capacity")]
    PublishTimeout,
}
