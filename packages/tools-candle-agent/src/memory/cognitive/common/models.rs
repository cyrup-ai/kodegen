//! Local Model Types
//!
//! Local-only model implementations for cognitive operations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Local model types for cognitive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalModelType {
    TextEvaluator,
    QualityAssessor,
    RelevanceScorer,
    ContentAnalyzer,
}

/// Local completion core error
#[derive(Error, Debug, Clone)]
pub enum CompletionCoreError {
    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Model error: {0}")]
    ModelError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Local model for cognitive operations
#[derive(Debug, Clone)]
pub struct LocalModel {
    pub model_type: LocalModelType,
    pub model_path: Option<String>,
    pub config: LocalModelConfig,
}

/// Configuration for local models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelConfig {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub local_only: bool,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            max_tokens: Some(512),
            temperature: Some(0.0), // Greedy sampling for cognitive operations - deterministic output
            local_only: true,
        }
    }
}
