//! Committee Evaluation System Using Existing Models
//!
//! Provides committee evaluation using existing CandleKimiK2Model and CandleQwen3CoderModel.

pub mod committee_evaluators;
pub mod committee_types;

// Re-export key types
pub use committee_evaluators::ModelCommitteeEvaluator;
pub use committee_types::{Committee, CommitteeConfig};
