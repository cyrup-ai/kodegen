//! Configuration types for the event bus system
//!
//! This module contains configuration structures for controlling event bus behavior,
//! including batching, metrics, and capacity settings.

/// Strategy for handling channel saturation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackpressureMode {
    /// Drop oldest events when channel is full (default, current behavior)
    /// Publishers never block, receivers may see `RecvError::Lagged`
    #[default]
    DropOldest,

    /// Block publisher until space is available
    /// Applies backpressure to slow down fast publishers
    /// WARNING: Can deadlock if all subscribers are slow
    Block,

    /// Return error when channel is full
    /// Publisher must handle `ChannelFull` error
    Error,
}

/// Configuration for the event bus
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Maximum number of events that can be buffered
    pub capacity: usize,

    /// Backpressure strategy when channel reaches capacity
    pub backpressure_mode: BackpressureMode,

    /// Pressure threshold (0.0-1.0) for `is_overloaded()` check
    /// Default 0.8 means warn when 80% full
    pub overload_threshold: f64,

    /// Whether to enable event batching for performance
    pub enable_batching: bool,
    /// Maximum batch size when batching is enabled
    pub max_batch_size: usize,
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
    /// Whether to enable event metrics collection
    pub enable_metrics: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            backpressure_mode: BackpressureMode::default(),
            overload_threshold: 0.8,
            enable_batching: false,
            max_batch_size: 100,
            batch_timeout_ms: 100,
            enable_metrics: true,
        }
    }
}
