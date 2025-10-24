//! SIMD-aligned activation patterns for vectorized neural processing

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// SIMD-aligned activation pattern for vectorized operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(align(32))] // Align to 32 bytes for AVX2 SIMD operations
pub struct AlignedActivationPattern {
    /// Activation values aligned for SIMD processing
    pub data: Vec<f32>,
    /// Pattern dimension for validation
    pub dimension: usize,
    /// Last update timestamp for decay calculations
    pub last_update: SystemTime,
}

impl AlignedActivationPattern {
    /// Create new aligned activation pattern
    #[inline]
    #[must_use]
    pub fn new(data: Vec<f32>) -> Self {
        let dimension = data.len();
        Self {
            data,
            dimension,
            last_update: SystemTime::now(),
        }
    }

    /// Update pattern with SIMD optimization hint
    #[inline]
    pub fn update(&mut self, new_data: Vec<f32>) {
        if new_data.len() == self.dimension {
            self.data = new_data;
            self.last_update = SystemTime::now();
        }
    }

    /// Apply activation function with SIMD optimization
    #[inline]
    pub fn apply_activation(&mut self, activation_fn: impl Fn(f32) -> f32) {
        for value in &mut self.data {
            *value = activation_fn(*value);
        }
        self.last_update = SystemTime::now();
    }

    /// Calculate pattern energy with SIMD hint
    #[inline]
    #[must_use]
    pub fn energy(&self) -> f32 {
        self.data.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    /// Check if the activation pattern is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Default for AlignedActivationPattern {
    #[inline]
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
