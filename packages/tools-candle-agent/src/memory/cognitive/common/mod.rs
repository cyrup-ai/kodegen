//! Common cognitive types and utilities
//!
//! Shared types for local cognitive operations.

pub mod models;
pub mod types;

// Re-export key types
pub use models::{CompletionCoreError, LocalModel, LocalModelType};
pub use types::{CommitteeConfig, EvaluationRubric, ImpactFactors};
