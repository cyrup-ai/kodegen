//! Error correction for quantum-inspired operations

use super::measurement::{MeasurementBasis, MeasurementMetadata};

/// Error correction configuration
#[derive(Debug, Clone)]
pub struct ErrorCorrectionConfig {
    pub measurement_basis: MeasurementBasis,
    pub measurement_metadata: MeasurementMetadata,
    pub correction_threshold: f64,
}

impl Default for ErrorCorrectionConfig {
    fn default() -> Self {
        Self {
            measurement_basis: MeasurementBasis::default(),
            measurement_metadata: MeasurementMetadata::default(),
            correction_threshold: 0.8,
        }
    }
}
