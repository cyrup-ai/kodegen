//! Message types and priorities for streaming

use super::super::events::RealTimeEvent;
use crate::domain::chat::message::types::{
    CandleMessage as Message, CandleMessageRole as MessageRole,
};
use crate::domain::util::unix_timestamp_nanos;
use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};

/// Live update message with zero-allocation string handling
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiveUpdateMessage {
    /// Message ID with zero-allocation storage
    pub id: String,
    /// Message content with zero-allocation storage
    pub content: String,
    /// Message type identifier
    pub message_type: String,
    /// Session ID with zero-allocation storage
    pub session_id: String,
    /// User ID with zero-allocation storage
    pub user_id: String,
    /// Timestamp in nanoseconds for high precision
    pub timestamp_nanos: u64,
    /// Message priority level
    pub priority: MessagePriority,
    /// Optional metadata with zero-allocation storage
    pub metadata: Option<String>,
    /// Message size in bytes for monitoring
    pub size_bytes: u32,
    /// Sequence number for ordering
    pub sequence_number: u64,
}

impl MessageChunk for LiveUpdateMessage {
    fn bad_chunk(error: String) -> Self {
        Self {
            id: String::new(),
            content: error,
            message_type: "error".to_string(),
            session_id: String::new(),
            user_id: String::new(),
            timestamp_nanos: 0,
            priority: MessagePriority::Normal,
            metadata: None,
            size_bytes: 0,
            sequence_number: 0,
        }
    }

    fn error(&self) -> Option<&str> {
        if self.message_type == "error" {
            Some(&self.content)
        } else {
            None
        }
    }
}

impl LiveUpdateMessage {
    /// Create a new live update message with current timestamp
    #[inline]
    #[must_use]
    pub fn new(
        id: String,
        content: String,
        message_type: String,
        session_id: String,
        user_id: String,
        priority: MessagePriority,
    ) -> Self {
        let timestamp_nanos = unix_timestamp_nanos();

        let size_bytes = u32::try_from(
            id.len() + content.len() + message_type.len() + session_id.len() + user_id.len(),
        )
        .unwrap_or(u32::MAX);

        Self {
            id,
            content,
            message_type,
            session_id,
            user_id,
            timestamp_nanos,
            priority,
            metadata: None,
            size_bytes,
            sequence_number: 0, // Will be set by streamer
        }
    }

    /// Add metadata to the message
    #[must_use]
    #[inline]
    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.size_bytes += u32::try_from(metadata.len()).unwrap_or(u32::MAX);
        self.metadata = Some(metadata);
        self
    }

    /// Get timestamp in seconds
    #[allow(clippy::cast_precision_loss)] // Acceptable for timestamp conversion
    #[inline]
    #[must_use]
    pub fn timestamp_seconds(&self) -> f64 {
        self.timestamp_nanos as f64 / 1_000_000_000.0
    }

    /// Get message age in nanoseconds
    #[inline]
    #[must_use]
    pub fn age_nanos(&self) -> u64 {
        let now_nanos = unix_timestamp_nanos();

        now_nanos.saturating_sub(self.timestamp_nanos)
    }

    /// Check if message has expired based on TTL
    #[inline]
    #[must_use]
    pub fn is_expired(&self, ttl_nanos: u64) -> bool {
        self.age_nanos() > ttl_nanos
    }

    /// Convert to `RealTimeEvent` for broadcasting
    #[must_use]
    pub fn to_real_time_event(&self) -> RealTimeEvent {
        // Create a basic Message for the event
        let message = Message::new(
            self.sequence_number,
            MessageRole::Assistant,
            self.content.as_bytes(),
        );

        RealTimeEvent::message_received(message, self.session_id.clone())
    }
}

/// Message priority levels with atomic-friendly ordering
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessagePriority {
    /// Low priority - background updates
    Low,
    /// Normal priority - standard messages
    Normal,
    /// High priority - important updates
    High,
    /// Critical priority - urgent notifications
    Critical,
}

impl MessagePriority {
    /// Get priority weight for ordering (higher = more important)
    #[inline]
    #[must_use]
    pub const fn weight(&self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Normal => 5,
            Self::High => 10,
            Self::Critical => 20,
        }
    }

    /// Convert to atomic representation
    #[inline]
    #[must_use]
    pub const fn to_atomic(&self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
            Self::Critical => 3,
        }
    }

    /// Convert from atomic representation
    #[inline]
    #[must_use]
    pub const fn from_atomic(value: u8) -> Self {
        match value {
            0 => Self::Low,
            1 => Self::Normal,
            2 => Self::High,
            _ => Self::Critical,
        }
    }
}

impl Default for MessagePriority {
    #[inline]
    fn default() -> Self {
        Self::Normal
    }
}
