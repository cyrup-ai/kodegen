//! Local Quantum-Inspired Cognitive Operations
//!
//! Quantum-inspired patterns for local cognitive processing without cloud dependencies.

pub mod entanglement;
pub mod error_correction;
pub mod measurement;
pub mod router;
pub mod state;
pub mod types;

// Re-export key types
pub use entanglement::EntanglementLink;
pub use measurement::{MeasurementBasis, MeasurementMetadata};
pub use router::{QuantumRouter, QuantumRouterError};
pub use state::QuantumState;
pub use types::{CognitiveError, EnhancedQuery, RoutingStrategy, TemporalContext, TemporalType};

/// Quantum signature for cognitive operations
#[derive(Debug, Clone)]
pub struct QuantumSignature {
    pub entanglement_id: String,
    pub coherence_level: f64,
    pub measurement_basis: String,
}

impl QuantumSignature {
    pub fn new() -> Self {
        Self {
            entanglement_id: "local-quantum".to_string(),
            coherence_level: 0.8,
            measurement_basis: "computational".to_string(),
        }
    }
}

impl Default for QuantumSignature {
    fn default() -> Self {
        Self::new()
    }
}
