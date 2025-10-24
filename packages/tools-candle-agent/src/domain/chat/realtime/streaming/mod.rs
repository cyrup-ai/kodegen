//! Live message streaming system with zero-allocation patterns
//!
//! This module provides high-performance message streaming using lock-free queues,
//! atomic counters, and tokio Stream patterns for blazing-fast real-time updates.

mod processing;
mod results;
mod stats;
mod streamer;
mod subscriber;
mod types;

// Public API exports
pub use streamer::LiveMessageStreamer;
pub use types::{LiveUpdateMessage, MessagePriority};
