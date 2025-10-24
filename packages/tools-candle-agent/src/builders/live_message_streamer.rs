//! Live message streamer builder implementations
//!
//! All live message streamer construction logic and builder patterns.

use crate::domain::chat::realtime::streaming::LiveMessageStreamer;

/// Live message streamer builder for ergonomic configuration
#[derive(Debug, Clone)]
pub struct LiveMessageStreamerBuilder {
    queue_size_limit: usize,
    backpressure_threshold: usize,
    processing_rate: u64,
    event_buffer_size: usize,
}

impl LiveMessageStreamerBuilder {
    /// Create new builder with defaults
    #[inline]
    pub fn new() -> Self {
        Self {
            queue_size_limit: 100000,        // Large queue for high throughput
            backpressure_threshold: 80000,   // 80% of queue size
            processing_rate: 10000,          // 10k messages/second
            event_buffer_size: 50000,        // Large event buffer
        }
    }

    /// Set queue size limit
    #[inline]
    pub fn queue_size_limit(mut self, limit: usize) -> Self {
        self.queue_size_limit = limit;
        self
    }

    /// Set backpressure threshold
    #[inline]
    pub fn backpressure_threshold(mut self, threshold: usize) -> Self {
        self.backpressure_threshold = threshold;
        self
    }

    /// Set processing rate (messages per second)
    #[inline]
    pub fn processing_rate(mut self, rate: u64) -> Self {
        self.processing_rate = rate;
        self
    }

    /// Set event buffer size
    #[inline]
    pub fn event_buffer_size(mut self, size: usize) -> Self {
        self.event_buffer_size = size;
        self
    }

    /// Build the live message streamer
    #[inline]
    pub fn build(self) -> LiveMessageStreamer {
        LiveMessageStreamer::new(
            self.queue_size_limit,
            self.backpressure_threshold,
            self.processing_rate,
        )
    }
}

impl Default for LiveMessageStreamerBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}