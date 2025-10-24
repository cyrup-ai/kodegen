//! Real-time event types with zero-allocation patterns
//!
//! This module provides comprehensive real-time event types using String for
//! zero-allocation string handling and atomic operations for blazing-fast performance.

//   // Removed unused import

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};

use crate::domain::chat::message::types::CandleMessage as Message;
use crate::domain::util::unix_timestamp_nanos;

/// Real-time event types with zero-allocation patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RealTimeEvent {
    /// User started typing
    TypingStarted {
        user_id: String,
        session_id: String,
        timestamp: u64,
    },
    /// User stopped typing
    TypingStopped {
        user_id: String,
        session_id: String,
        timestamp: u64,
    },
    /// New message received
    MessageReceived {
        message: Message,
        session_id: String,
        timestamp: u64,
    },
    /// Message updated
    MessageUpdated {
        message_id: String,
        content: String,
        session_id: String,
        timestamp: u64,
    },
    /// Message deleted
    MessageDeleted {
        message_id: String,
        session_id: String,
        timestamp: u64,
    },
    /// User joined session
    UserJoined {
        user_id: String,
        session_id: String,
        timestamp: u64,
    },
    /// User left session
    UserLeft {
        user_id: String,
        session_id: String,
        timestamp: u64,
    },
    /// Connection status changed
    ConnectionStatusChanged {
        user_id: String,
        status: ConnectionStatus,
        timestamp: u64,
    },
    /// Heartbeat received
    HeartbeatReceived {
        user_id: String,
        session_id: String,
        timestamp: u64,
    },
    /// System notification
    SystemNotification {
        message: String,
        level: NotificationLevel,
        timestamp: u64,
    },
}

impl Default for RealTimeEvent {
    fn default() -> Self {
        RealTimeEvent::HeartbeatReceived {
            user_id: String::new(),
            session_id: String::new(),
            timestamp: 0,
        }
    }
}

impl MessageChunk for RealTimeEvent {
    fn bad_chunk(error: String) -> Self {
        RealTimeEvent::SystemNotification {
            message: error,
            level: NotificationLevel::Error,
            timestamp: Self::current_timestamp(),
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            RealTimeEvent::SystemNotification {
                message,
                level: NotificationLevel::Error,
                ..
            } => Some(message),
            _ => None,
        }
    }
}

impl RealTimeEvent {
    /// Get current timestamp in nanoseconds for zero-allocation timing
    #[inline]
    #[must_use]
    pub fn current_timestamp() -> u64 {
        unix_timestamp_nanos()
    }

    /// Create typing started event with current timestamp
    #[inline]
    #[must_use]
    pub fn typing_started(user_id: String, session_id: String) -> Self {
        Self::TypingStarted {
            user_id,
            session_id,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create typing stopped event with current timestamp
    #[inline]
    #[must_use]
    pub fn typing_stopped(user_id: String, session_id: String) -> Self {
        Self::TypingStopped {
            user_id,
            session_id,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create message received event with current timestamp
    #[inline]
    #[must_use]
    pub fn message_received(message: Message, session_id: String) -> Self {
        Self::MessageReceived {
            message,
            session_id,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create message updated event with current timestamp
    #[inline]
    #[must_use]
    pub fn message_updated(message_id: String, content: String, session_id: String) -> Self {
        Self::MessageUpdated {
            message_id,
            content,
            session_id,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create message deleted event with current timestamp
    #[inline]
    #[must_use]
    pub fn message_deleted(message_id: String, session_id: String) -> Self {
        Self::MessageDeleted {
            message_id,
            session_id,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create user joined event with current timestamp
    #[inline]
    #[must_use]
    pub fn user_joined(user_id: String, session_id: String) -> Self {
        Self::UserJoined {
            user_id,
            session_id,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create user left event with current timestamp
    #[inline]
    #[must_use]
    pub fn user_left(user_id: String, session_id: String) -> Self {
        Self::UserLeft {
            user_id,
            session_id,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create connection status changed event with current timestamp
    #[inline]
    #[must_use]
    pub fn connection_status_changed(user_id: String, status: ConnectionStatus) -> Self {
        Self::ConnectionStatusChanged {
            user_id,
            status,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create heartbeat received event with current timestamp
    #[inline]
    #[must_use]
    pub fn heartbeat_received(user_id: String, session_id: String) -> Self {
        Self::HeartbeatReceived {
            user_id,
            session_id,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Create system notification event with current timestamp
    #[inline]
    #[must_use]
    pub fn system_notification(message: String, level: NotificationLevel) -> Self {
        Self::SystemNotification {
            message,
            level,
            timestamp: Self::current_timestamp(),
        }
    }

    /// Get event type name for zero-allocation logging
    #[inline]
    #[must_use]
    pub const fn event_type(&self) -> &'static str {
        match self {
            Self::TypingStarted { .. } => "TypingStarted",
            Self::TypingStopped { .. } => "TypingStopped",
            Self::MessageReceived { .. } => "MessageReceived",
            Self::MessageUpdated { .. } => "MessageUpdated",
            Self::MessageDeleted { .. } => "MessageDeleted",
            Self::UserJoined { .. } => "UserJoined",
            Self::UserLeft { .. } => "UserLeft",
            Self::ConnectionStatusChanged { .. } => "ConnectionStatusChanged",
            Self::HeartbeatReceived { .. } => "HeartbeatReceived",
            Self::SystemNotification { .. } => "SystemNotification",
        }
    }

    /// Get timestamp for any event type
    #[inline]
    #[must_use]
    pub const fn timestamp(&self) -> u64 {
        match self {
            Self::TypingStarted { timestamp, .. }
            | Self::TypingStopped { timestamp, .. }
            | Self::MessageReceived { timestamp, .. }
            | Self::MessageUpdated { timestamp, .. }
            | Self::MessageDeleted { timestamp, .. }
            | Self::UserJoined { timestamp, .. }
            | Self::UserLeft { timestamp, .. }
            | Self::ConnectionStatusChanged { timestamp, .. }
            | Self::HeartbeatReceived { timestamp, .. }
            | Self::SystemNotification { timestamp, .. } => *timestamp,
        }
    }
}

/// Connection status enumeration with atomic-friendly operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConnectionStatus {
    /// Connected and active
    Connected,
    /// Connecting
    Connecting,
    /// Disconnected
    Disconnected,
    /// Connection error
    Error,
    /// Reconnecting
    Reconnecting,
    /// Connection failed
    Failed,
    /// Idle (connected but inactive)
    Idle,
    /// Unstable connection
    Unstable,
}

impl ConnectionStatus {
    /// Convert to atomic representation (u8) for lock-free storage
    #[inline]
    #[must_use]
    pub const fn to_atomic(&self) -> u8 {
        match self {
            Self::Connected => 0,
            Self::Connecting => 1,
            Self::Disconnected => 2,
            Self::Error => 3,
            Self::Reconnecting => 4,
            Self::Failed => 5,
            Self::Idle => 6,
            Self::Unstable => 7,
        }
    }

    /// Convert from atomic representation (u8) for lock-free retrieval
    #[inline]
    #[must_use]
    pub const fn from_atomic(value: u8) -> Self {
        match value {
            0 => Self::Connected,
            1 => Self::Connecting,
            2 => Self::Disconnected,
            3 => Self::Error,
            4 => Self::Reconnecting,
            5 => Self::Failed,
            6 => Self::Idle,
            _ => Self::Unstable, // Default fallback for invalid values
        }
    }

    /// Check if connection is active (connected or idle)
    #[inline]
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Connected | Self::Idle)
    }

    /// Check if connection is in error state
    #[inline]
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error | Self::Failed)
    }

    /// Check if connection is transitioning
    #[inline]
    #[must_use]
    pub const fn is_transitioning(&self) -> bool {
        matches!(self, Self::Connecting | Self::Reconnecting)
    }

    /// Get status priority for connection management (higher = more important)
    #[inline]
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Connected => 100,
            Self::Idle => 90,
            Self::Connecting => 80,
            Self::Reconnecting => 70,
            Self::Unstable => 60,
            Self::Disconnected => 40,
            Self::Error => 20,
            Self::Failed => 10,
        }
    }
}

impl Default for ConnectionStatus {
    #[inline]
    fn default() -> Self {
        Self::Disconnected
    }
}

/// Notification level enumeration for system messages
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NotificationLevel {
    /// Information message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
    /// Success message
    Success,
    /// Debug message (development only)
    Debug,
    /// Critical system message
    Critical,
}

impl NotificationLevel {
    /// Get level priority for filtering (higher = more important)
    #[inline]
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Critical => 100,
            Self::Error => 80,
            Self::Warning => 60,
            Self::Success => 40,
            Self::Info => 20,
            Self::Debug => 10,
        }
    }

    /// Get level name for zero-allocation logging
    #[inline]
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
            Self::Success => "SUCCESS",
            Self::Debug => "DEBUG",
            Self::Critical => "CRITICAL",
        }
    }

    /// Check if level should be displayed in production
    #[inline]
    #[must_use]
    pub const fn is_production_level(&self) -> bool {
        !matches!(self, Self::Debug)
    }
}

impl Default for NotificationLevel {
    #[inline]
    fn default() -> Self {
        Self::Info
    }
}

/// Event filtering criteria for zero-allocation event routing
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by session ID
    pub session_id: Option<String>,
    /// Filter by event types
    pub event_types: Option<Vec<&'static str>>,
    /// Filter by minimum notification level
    pub min_notification_level: Option<NotificationLevel>,
    /// Filter by timestamp range
    pub timestamp_range: Option<(u64, u64)>,
}

impl EventFilter {
    /// Create new event filter with no restrictions
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            user_id: None,
            session_id: None,
            event_types: None,
            min_notification_level: None,
            timestamp_range: None,
        }
    }

    /// Filter by user ID
    #[must_use]
    #[inline]
    pub fn user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Filter by session ID
    #[must_use]
    #[inline]
    pub fn session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Filter by event types
    #[must_use]
    #[inline]
    pub fn event_types(mut self, types: Vec<&'static str>) -> Self {
        self.event_types = Some(types);
        self
    }

    /// Filter by minimum notification level
    #[must_use]
    #[inline]
    pub fn min_notification_level(mut self, level: NotificationLevel) -> Self {
        self.min_notification_level = Some(level);
        self
    }

    /// Filter by timestamp range
    #[must_use]
    #[inline]
    pub fn timestamp_range(mut self, start: u64, end: u64) -> Self {
        self.timestamp_range = Some((start, end));
        self
    }

    /// Check if event matches this filter
    #[must_use]
    pub fn matches(&self, event: &RealTimeEvent) -> bool {
        // Check user ID filter
        if let Some(filter_user_id) = &self.user_id {
            let event_user_id = match event {
                RealTimeEvent::TypingStarted { user_id, .. }
                | RealTimeEvent::TypingStopped { user_id, .. }
                | RealTimeEvent::UserJoined { user_id, .. }
                | RealTimeEvent::UserLeft { user_id, .. }
                | RealTimeEvent::ConnectionStatusChanged { user_id, .. }
                | RealTimeEvent::HeartbeatReceived { user_id, .. } => Some(user_id),
                _ => None,
            };

            if let Some(user_id) = event_user_id
                && user_id != filter_user_id
            {
                return false;
            }
        }

        // Check session ID filter
        if let Some(filter_session_id) = &self.session_id {
            let event_session_id = match event {
                RealTimeEvent::TypingStarted { session_id, .. }
                | RealTimeEvent::TypingStopped { session_id, .. }
                | RealTimeEvent::MessageReceived { session_id, .. }
                | RealTimeEvent::MessageUpdated { session_id, .. }
                | RealTimeEvent::MessageDeleted { session_id, .. }
                | RealTimeEvent::UserJoined { session_id, .. }
                | RealTimeEvent::UserLeft { session_id, .. }
                | RealTimeEvent::HeartbeatReceived { session_id, .. } => Some(session_id),
                _ => None,
            };

            if let Some(session_id) = event_session_id
                && session_id != filter_session_id
            {
                return false;
            }
        }

        // Check event type filter
        if let Some(filter_types) = &self.event_types
            && !filter_types.contains(&event.event_type())
        {
            return false;
        }

        // Check notification level filter
        if let Some(min_level) = &self.min_notification_level
            && let RealTimeEvent::SystemNotification { level, .. } = event
            && level.priority() < min_level.priority()
        {
            return false;
        }

        // Check timestamp range filter
        if let Some((start, end)) = &self.timestamp_range {
            let timestamp = event.timestamp();
            if timestamp < *start || timestamp > *end {
                return false;
            }
        }

        true
    }
}

impl Default for EventFilter {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
