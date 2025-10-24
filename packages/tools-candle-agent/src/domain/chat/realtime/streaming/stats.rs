//! Streaming statistics

use serde::{Deserialize, Serialize};

/// Live streaming statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingStatistics {
    /// Total messages in normal queue
    pub total_messages: usize,
    /// Total messages in priority queue
    pub priority_messages: usize,
    /// Number of active subscribers
    pub active_subscribers: usize,
    /// Total subscribers ever created
    pub total_subscribers: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Queue size limit
    pub queue_size_limit: usize,
    /// Number of backpressure events
    pub backpressure_events: u64,
    /// Target processing rate (messages/second)
    pub processing_rate: u64,
    /// Whether processing is active
    pub processing_active: bool,
}
