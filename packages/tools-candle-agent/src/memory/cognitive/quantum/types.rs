//! Quantum-inspired types for local cognitive operations

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Quantum routing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingStrategy {
    Quantum,
    Attention,
    Causal,
    Emergent,
    Hybrid(Vec<RoutingStrategy>),
}

/// Enhanced query with quantum-inspired features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedQuery {
    pub query: String,
    pub routing_strategy: RoutingStrategy,
    pub temporal_context: TemporalContext,
    pub coherence_threshold: f64,
}

/// Temporal context for quantum operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    pub temporal_type: TemporalType,
    pub time_window: u64,
    pub decay_factor: f64,
}

impl Default for TemporalContext {
    fn default() -> Self {
        Self {
            temporal_type: TemporalType::Present,
            time_window: 3600, // 1 hour
            decay_factor: 0.1,
        }
    }
}
/// Temporal type for quantum operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalType {
    Past,
    Present,
    Future,
}

/// Cognitive error types
#[derive(Error, Debug, Clone)]
pub enum CognitiveError {
    #[error("Invalid quantum state: {0}")]
    InvalidQuantumState(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Routing error: {0}")]
    RoutingError(String),

    #[error("Measurement error: {0}")]
    MeasurementError(String),
}

/// Routing decision for quantum operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub strategy: RoutingStrategy,
    pub confidence: f64,
    pub reasoning: String,
}

impl Default for RoutingDecision {
    fn default() -> Self {
        Self {
            strategy: RoutingStrategy::Attention,
            confidence: 0.7,
            reasoning: "Default routing decision".to_string(),
        }
    }
}
