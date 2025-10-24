//! Quantum measurement simulation for local operations

use serde::{Deserialize, Serialize};

/// Measurement basis for quantum operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MeasurementBasis {
    #[default]
    Computational,
    Hadamard,
    Bell,
    Custom(String),
}

/// Measurement metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementMetadata {
    pub basis: MeasurementBasis,
    pub timestamp: u64,
    pub confidence: f64,
    pub coherence_time: f64,
}

impl Default for MeasurementMetadata {
    fn default() -> Self {
        Self {
            basis: MeasurementBasis::default(),
            timestamp: 0,
            confidence: 0.8,
            coherence_time: 1.0,
        }
    }
}

/// Quantum measurement simulator
#[derive(Debug, Clone)]
pub struct QuantumMeasurement {
    pub basis: MeasurementBasis,
    pub metadata: MeasurementMetadata,
}

impl QuantumMeasurement {
    pub fn new(basis: MeasurementBasis) -> Self {
        Self {
            basis: basis.clone(),
            metadata: MeasurementMetadata {
                basis,
                ..Default::default()
            },
        }
    }

    pub fn measure(&self, _state: &str) -> f64 {
        // Simple measurement simulation
        0.8 // Fixed value for local operation
    }
}
