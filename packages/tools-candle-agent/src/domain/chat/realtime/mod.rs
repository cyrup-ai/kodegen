//! Real-time chat functionality
//!
//! This module provides real-time chat features including:
//! - Connection management
//! - Message streaming
//! - Typing indicators
//! - Event broadcasting

mod connection;
mod events;
mod streaming;
mod system;
mod typing;

// Re-export public interfaces
pub use connection::{ConnectionManager, ConnectionState, ConnectionStatistics};
pub use events::{ConnectionStatus, EventFilter, NotificationLevel, RealTimeEvent};
pub use streaming::{LiveMessageStreamer, LiveUpdateMessage, MessagePriority};
pub use system::{RealTimeSystem, RealtimeChat, RealtimeConfig};
pub use typing::{TypingIndicator, TypingStatistics};

/// Real-time error type
#[derive(Debug, thiserror::Error)]
pub enum RealTimeError {
    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Message error
    #[error("Message error: {0}")]
    MessageError(String),

    /// Subscription error
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    /// System error
    #[error("System error: {0}")]
    SystemError(String),

    /// Timeout error
    #[error("Operation timed out")]
    TimeoutError,

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type for real-time operations
pub type Result<T> = std::result::Result<T, RealTimeError>;
