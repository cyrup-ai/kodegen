//! Background worker for proactive temporal decay
//!
//! Processes memories and their relationships in batches, applying exponential decay to:
//! - Memory importance scores
//! - Quantum coherence levels
//! - Entanglement edge strengths
//! - Causal edge strengths
//!
//! This eliminates expensive on-read decay calculations from the hot path.

mod config;
mod worker;

pub use config::DecayWorkerConfig;
pub use worker::DecayWorker;
