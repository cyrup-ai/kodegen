//! Local Cognitive Types
//!
//! Core types for local cognitive memory operations without cloud dependencies.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Local cognitive error types
#[derive(Error, Debug, Clone)]
pub enum CognitiveError {
    #[error("Invalid quantum state: {0}")]
    InvalidQuantumState(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Local model error: {0}")]
    LocalModelError(String),

    #[error("Initialization error: {0}")]
    InitializationError(String),

    #[error("Evaluation error: {0}")]
    EvaluationError(String),
}

/// Local cognitive state for memory processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveState {
    pub semantic_context: SemanticContext,
    pub temporal_context: TemporalContext,
    pub confidence_level: f64,
}

impl Default for CognitiveState {
    fn default() -> Self {
        Self {
            semantic_context: SemanticContext::default(),
            temporal_context: TemporalContext::default(),
            confidence_level: 0.5,
        }
    }
}

impl CognitiveState {
    pub fn new(semantic_context: SemanticContext) -> Self {
        Self {
            semantic_context,
            temporal_context: TemporalContext::default(),
            confidence_level: 0.5,
        }
    }
}
/// Semantic context for cognitive processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContext {
    pub domain: String,
    pub abstraction_level: AbstractionLevel,
    pub relevance_score: f64,
}

impl Default for SemanticContext {
    fn default() -> Self {
        Self {
            domain: "general".to_string(),
            abstraction_level: AbstractionLevel::Intermediate,
            relevance_score: 0.5,
        }
    }
}

/// Temporal context for memory operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    pub temporal_type: TemporalType,
    pub time_horizon: u64,
    pub decay_factor: f64,
}

impl Default for TemporalContext {
    fn default() -> Self {
        Self {
            temporal_type: TemporalType::Present,
            time_horizon: 86400, // 1 day in seconds
            decay_factor: 0.1,
        }
    }
}

/// Temporal type for cognitive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemporalType {
    Past,
    Present,
    Future,
}

/// Abstraction level for semantic processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbstractionLevel {
    Low,
    Intermediate,
    High,
}
