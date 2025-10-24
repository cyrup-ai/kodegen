//! Entanglement simulation for local operations

use serde::{Deserialize, Serialize};

/// Entanglement link between cognitive nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntanglementLink {
    pub node_a: String,
    pub node_b: String,
    pub entanglement_strength: f64,
    pub coherence_time: f64,
}

impl EntanglementLink {
    pub fn new(node_a: String, node_b: String, strength: f64) -> Self {
        Self {
            node_a,
            node_b,
            entanglement_strength: strength,
            coherence_time: 1.0,
        }
    }

    pub fn measure_correlation(&self) -> f64 {
        self.entanglement_strength
    }
}
