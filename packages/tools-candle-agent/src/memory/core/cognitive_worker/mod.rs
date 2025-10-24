//! Cognitive task processing worker
//!
//! This module provides background processing for memory cognitive tasks including:
//! - Pattern emergence detection
//! - Committee-based quality evaluation
//! - Quantum entanglement discovery
//! - Temporal context maintenance
//!
//! The worker processes tasks asynchronously from a queue, delegating to specialized
//! handlers for different cognitive operations.

mod committee_evaluation;
mod entanglement_discovery;
mod temporal_maintenance;
mod worker_core;

// Re-export public API
pub use worker_core::CognitiveWorker;
