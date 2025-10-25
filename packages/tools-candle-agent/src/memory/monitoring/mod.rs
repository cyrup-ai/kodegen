//! Monitoring module for mem0-rs
//!
//! This module provides system monitoring, health checks, metrics collection,
//! and performance tracking for the memory system.

pub mod health;
pub mod memory_usage;
pub mod metrics;
pub mod monitor;
pub mod operations;
pub mod performance;

// Internal fallback logic (not exported publicly)
pub(crate) mod fallback;

// Re-export main types
pub use health::*;
pub use memory_usage::*;
pub use metrics::*;
pub use monitor::*;
pub use operations::*;
pub use performance::*;
