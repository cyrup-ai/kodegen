//! Database schema for quantum cognitive signatures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::memory::cognitive::types::{
    AlignedCoherenceFingerprint, CognitiveState, EntanglementBond, EntanglementType,
    QuantumSignature,
};

/// Database schema for quantum signature (denormalized cache)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumSignatureSchema {
    /// Coherence fingerprint for quantum states
    pub coherence_fingerprint: CoherenceFingerprintSchema,

    /// Entanglement bonds (cached snapshot)
    pub entanglement_bonds: Vec<EntanglementBondSchema>,

    /// Superposition contexts
    pub superposition_contexts: Vec<String>,

    /// Collapse probability
    pub collapse_probability: f32,

    /// Quantum entropy
    pub quantum_entropy: f64,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Decoherence rate
    pub decoherence_rate: f64,
}

/// Schema for coherence fingerprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceFingerprintSchema {
    pub amplitudes: Vec<f32>,
    pub phases: Vec<f32>,
    pub dimension: usize,
}

/// Schema for entanglement bond
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntanglementBondSchema {
    pub target_id: String, // Uuid as string for JSON
    pub bond_strength: f32,
    pub bond_type: String, // EntanglementType as string
    pub created_at: DateTime<Utc>,
}

impl QuantumSignatureSchema {
    /// Convert from CognitiveState's quantum signature
    pub async fn from_cognitive_state(state: &CognitiveState) -> Self {
        let signature = state.quantum_signature();

        // Extract coherence fingerprint
        let fingerprint = signature.coherence_fingerprint();
        let coherence_fingerprint = CoherenceFingerprintSchema {
            amplitudes: fingerprint.amplitudes.clone(),
            phases: fingerprint.phases.clone(),
            dimension: fingerprint.dimension,
        };

        // Extract entanglement bonds
        let entanglement_bonds = signature
            .entanglement_bonds()
            .await
            .iter()
            .map(|bond| EntanglementBondSchema {
                target_id: bond.target_id.to_string(),
                bond_strength: bond.bond_strength,
                bond_type: format!("{:?}", bond.entanglement_type),
                created_at: bond.created_at.into(),
            })
            .collect();

        // Extract other fields
        let superposition_contexts = signature
            .superposition_contexts()
            .iter()
            .map(|ctx| ctx.to_string())
            .collect();

        Self {
            coherence_fingerprint,
            entanglement_bonds,
            superposition_contexts,
            collapse_probability: signature.collapse_probability(),
            quantum_entropy: signature.quantum_entropy(),
            created_at: signature.creation_time().into(),
            decoherence_rate: signature.decoherence_rate(),
        }
    }

    /// Convert to CognitiveState's quantum signature
    pub fn to_cognitive_state(&self) -> Result<CognitiveState, String> {
        // Create AlignedCoherenceFingerprint
        let fingerprint = AlignedCoherenceFingerprint::new(
            self.coherence_fingerprint.amplitudes.clone(),
            self.coherence_fingerprint.phases.clone(),
        )
        .map_err(|e| format!("Failed to create coherence fingerprint: {}", e))?;

        // Parse entanglement bonds
        let bonds: Result<Vec<EntanglementBond>, String> = self
            .entanglement_bonds
            .iter()
            .map(|bond_schema| {
                let target_id = Uuid::parse_str(&bond_schema.target_id)
                    .map_err(|e| format!("Invalid UUID: {}", e))?;

                let entanglement_type = match bond_schema.bond_type.as_str() {
                    "Semantic" => EntanglementType::Semantic,
                    "Temporal" => EntanglementType::Temporal,
                    "Causal" => EntanglementType::Causal,
                    "Emergent" => EntanglementType::Emergent,
                    "Werner" => EntanglementType::Werner,
                    "Weak" => EntanglementType::Weak,
                    "Bell" => EntanglementType::Bell,
                    "BellPair" => EntanglementType::BellPair,
                    _ => {
                        return Err(format!(
                            "Unknown entanglement type: {}",
                            bond_schema.bond_type
                        ));
                    }
                };

                Ok(EntanglementBond {
                    target_id,
                    bond_strength: bond_schema.bond_strength,
                    entanglement_type,
                    created_at: bond_schema.created_at.into(),
                })
            })
            .collect();

        let bonds = bonds?;

        // Parse superposition contexts
        let superposition_contexts: Vec<std::sync::Arc<str>> = self
            .superposition_contexts
            .iter()
            .map(|s| std::sync::Arc::from(s.as_str()))
            .collect();

        // Create QuantumSignature
        let quantum_signature = QuantumSignature::new_with_data(
            fingerprint,
            bonds,
            superposition_contexts,
            self.collapse_probability,
            self.quantum_entropy,
            self.created_at.into(),
            self.decoherence_rate,
        );

        // Create CognitiveState with quantum signature
        Ok(CognitiveState::new_with_quantum_signature(
            quantum_signature,
        ))
    }
}
