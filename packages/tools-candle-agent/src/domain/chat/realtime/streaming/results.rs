//! Streaming operation results and events

use super::types::MessagePriority;
use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};

/// Streaming operation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingResult {
    /// Message successfully queued
    MessageQueued {
        message_id: String,
        sequence_number: u64,
        queue_position: usize,
    },
    /// Backpressure warning - queue size approaching limit
    BackpressureWarning {
        current_size: usize,
        threshold: usize,
        message_id: String,
    },
    /// Backpressure error occurred
    BackpressureError {
        current_size: usize,
        limit: usize,
        message_id: String,
    },
}

impl Default for StreamingResult {
    fn default() -> Self {
        StreamingResult::MessageQueued {
            message_id: String::new(),
            sequence_number: 0,
            queue_position: 0,
        }
    }
}

impl MessageChunk for StreamingResult {
    fn bad_chunk(error: String) -> Self {
        StreamingResult::BackpressureError {
            current_size: 0,
            limit: 0,
            message_id: error,
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            StreamingResult::BackpressureError { message_id, .. } => Some(message_id),
            _ => None,
        }
    }
}

/// Unsubscribe operation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnsubscribeResult {
    /// Successfully unsubscribed
    Success {
        subscriber_id: String,
        remaining_subscribers: u64,
    },
    /// Subscriber not found
    NotFound { subscriber_id: String },
}

impl Default for UnsubscribeResult {
    fn default() -> Self {
        UnsubscribeResult::Success {
            subscriber_id: String::new(),
            remaining_subscribers: 0,
        }
    }
}

impl MessageChunk for UnsubscribeResult {
    fn bad_chunk(error: String) -> Self {
        UnsubscribeResult::NotFound {
            subscriber_id: error,
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            UnsubscribeResult::NotFound { subscriber_id } => Some(subscriber_id),
            UnsubscribeResult::Success { .. } => None,
        }
    }
}

/// Processing events for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingEvent {
    /// Message processed and distributed
    MessageProcessed {
        message_id: String,
        sequence_number: u64,
        delivered_count: u64,
        total_bytes: u64,
        priority: MessagePriority,
    },
    /// Processing rate report
    RateReport {
        messages_per_second: f64,
        messages_processed: u64,
        active_subscribers: u64,
    },
}

impl Default for ProcessingEvent {
    fn default() -> Self {
        ProcessingEvent::RateReport {
            messages_per_second: 0.0,
            messages_processed: 0,
            active_subscribers: 0,
        }
    }
}

impl MessageChunk for ProcessingEvent {
    fn bad_chunk(error: String) -> Self {
        Self::MessageProcessed {
            message_id: error,
            sequence_number: 0,
            delivered_count: 0,
            total_bytes: 0,
            priority: MessagePriority::Normal,
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            Self::MessageProcessed {
                message_id,
                delivered_count,
                ..
            } if *delivered_count == 0 => Some(message_id),
            _ => None,
        }
    }
}
