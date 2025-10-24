//! Quantum-inspired signatures and entanglement for cognitive memory routing

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

use kodegen_simd::similarity::cosine_similarity;

use super::atomics::{AtomicF32, AtomicF64};
use crate::domain::memory::cognitive::types::state::{CognitiveError, CognitiveResult};

/// Quantum-inspired signature for entanglement-based routing
#[derive(Debug, Clone)]
pub struct QuantumSignature {
    /// SIMD-aligned coherence fingerprint for quantum state
    coherence_fingerprint: AlignedCoherenceFingerprint,
    /// Entanglement bonds with other quantum signatures
    entanglement_bonds: Arc<RwLock<Vec<EntanglementBond>>>,
    /// Superposition contexts for multi-state representation
    superposition_contexts: Vec<Arc<str>>,
    /// Collapse probability
    collapse_probability: Arc<AtomicF32>,
    /// Quantum entropy measure
    quantum_entropy: Arc<AtomicF64>,
    /// Creation timestamp for decoherence
    creation_time: SystemTime,
    /// Decoherence rate (per second)
    decoherence_rate: f64,
}

/// SIMD-aligned coherence fingerprint for quantum state representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(align(32))] // Align to 32 bytes for AVX2 SIMD operations
pub struct AlignedCoherenceFingerprint {
    /// Amplitude values for quantum states
    pub amplitudes: Vec<f32>,
    /// Phase angles for quantum interference
    pub phases: Vec<f32>,
    /// Dimension for consistency checking
    pub dimension: usize,
}

impl AlignedCoherenceFingerprint {
    /// Create new coherence fingerprint
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if amplitudes and phases vectors have different dimensions
    #[inline]
    pub fn new(amplitudes: Vec<f32>, phases: Vec<f32>) -> Result<Self, CognitiveError> {
        if amplitudes.len() != phases.len() {
            return Err(CognitiveError::InvalidQuantumState(
                "Amplitudes and phases must have same dimension".to_string(),
            ));
        }

        let dimension = amplitudes.len();
        Ok(Self {
            amplitudes,
            phases,
            dimension,
        })
    }

    /// Check if fingerprint is empty
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.amplitudes.is_empty()
    }

    /// Calculate state probability using quantum Born rule
    #[inline]
    #[must_use]
    pub fn state_probability(&self) -> f32 {
        // |ψ|² = sum of squared amplitudes
        self.amplitudes.iter().map(|x| x * x).sum::<f32>()
    }

    /// Apply quantum gate operation
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if gate matrix dimensions don't match fingerprint dimension
    #[inline]
    pub fn apply_gate(&mut self, gate_matrix: &[f32]) -> Result<(), CognitiveError> {
        if gate_matrix.len() != self.dimension * self.dimension {
            return Err(CognitiveError::InvalidQuantumOperation(format!(
                "Gate matrix size {} doesn't match fingerprint dimension {}²",
                gate_matrix.len(),
                self.dimension
            )));
        }

        let mut new_amplitudes = vec![0.0; self.dimension];
        for i in 0..self.dimension {
            for j in 0..self.dimension {
                new_amplitudes[i] += gate_matrix[i * self.dimension + j] * self.amplitudes[j];
            }
        }

        self.amplitudes = new_amplitudes;
        Ok(())
    }

    /// Measure entanglement between two coherence fingerprints
    #[inline]
    #[must_use]
    pub fn entanglement_measure(&self, other: &Self) -> Option<f32> {
        if self.dimension != other.dimension {
            return None;
        }

        if self.is_empty() || other.is_empty() {
            return None;
        }

        // Use SIMD-optimized cosine similarity as entanglement measure
        Some(cosine_similarity(&self.amplitudes, &other.amplitudes))
    }
}

impl Default for AlignedCoherenceFingerprint {
    #[inline]
    fn default() -> Self {
        Self {
            amplitudes: Vec::new(),
            phases: Vec::new(),
            dimension: 0,
        }
    }
}

impl QuantumSignature {
    /// Create new quantum signature with default parameters
    #[must_use]
    pub fn new() -> Self {
        Self {
            coherence_fingerprint: AlignedCoherenceFingerprint::default(),
            entanglement_bonds: Arc::new(RwLock::new(Vec::new())),
            superposition_contexts: Vec::new(),
            collapse_probability: Arc::new(AtomicF32::new(0.5)),
            quantum_entropy: Arc::new(AtomicF64::new(0.0)),
            creation_time: SystemTime::now(),
            decoherence_rate: 0.001, // Default 0.1% per second
        }
    }

    /// Create quantum signature with custom coherence state
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if amplitudes and phases have different dimensions
    #[inline]
    pub fn with_coherence(amplitudes: &[f32], phases: Vec<f32>) -> Result<Self, CognitiveError> {
        // Create coherence fingerprint with validation
        let coherence_fingerprint =
            AlignedCoherenceFingerprint::new(amplitudes.to_owned(), phases)?;

        // Calculate initial collapse probability from amplitudes
        let collapse_probability = amplitudes.iter().map(|x| x * x).sum::<f32>();

        // Calculate initial entropy: H = -Σ p_i log(p_i)
        // Use f64 accumulation to preserve precision for AtomicF64 storage
        let quantum_entropy = -amplitudes
            .iter()
            .map(|&a| {
                let p = f64::from(a * a);
                if p > 0.0 { p * p.ln() } else { 0.0 }
            })
            .sum::<f64>();

        let superposition_contexts = Vec::new();
        let creation_time = SystemTime::now();
        let decoherence_rate = 0.001;

        Ok(Self {
            coherence_fingerprint,
            entanglement_bonds: Arc::new(RwLock::new(Vec::new())),
            superposition_contexts,
            collapse_probability: Arc::new(AtomicF32::new(collapse_probability)),
            quantum_entropy: Arc::new(AtomicF64::new(quantum_entropy)),
            creation_time,
            decoherence_rate,
        })
    }

    /// Create quantum signature with full data
    #[inline]
    #[must_use]
    pub fn new_with_data(
        coherence_fingerprint: AlignedCoherenceFingerprint,
        entanglement_bonds: Vec<EntanglementBond>,
        superposition_contexts: Vec<Arc<str>>,
        collapse_probability: f32,
        quantum_entropy: f64,
        creation_time: SystemTime,
        decoherence_rate: f64,
    ) -> Self {
        Self {
            coherence_fingerprint,
            entanglement_bonds: Arc::new(RwLock::new(entanglement_bonds)),
            superposition_contexts,
            collapse_probability: Arc::new(AtomicF32::new(collapse_probability)),
            quantum_entropy: Arc::new(AtomicF64::new(quantum_entropy)),
            creation_time,
            decoherence_rate,
        }
    }

    /// Apply decoherence based on elapsed time
    #[inline]
    pub fn apply_decoherence(&self) {
        let elapsed = self
            .creation_time
            .elapsed()
            .unwrap_or(std::time::Duration::ZERO);
        let decoherence_factor = (-self.decoherence_rate * elapsed.as_secs_f64()).exp();

        let current_entropy = self.quantum_entropy.load(Ordering::Relaxed);
        let new_entropy = current_entropy + (1.0 - decoherence_factor);
        self.quantum_entropy.store(new_entropy, Ordering::Relaxed);
    }

    /// Get collapse probability
    #[inline]
    #[must_use]
    pub fn collapse_probability(&self) -> f32 {
        self.collapse_probability.load(Ordering::Relaxed)
    }

    /// Set collapse probability
    #[inline]
    pub fn set_collapse_probability(&self, probability: f32) {
        self.collapse_probability
            .store(probability.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Get quantum entropy
    #[inline]
    #[must_use]
    pub fn quantum_entropy(&self) -> f64 {
        self.quantum_entropy.load(Ordering::Relaxed)
    }

    /// Get coherence state probability using quantum mechanics
    #[inline]
    #[must_use]
    pub fn coherence_state_probability(&self) -> f32 {
        self.coherence_fingerprint.state_probability()
    }

    /// Apply quantum gate operation to coherence fingerprint
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if gate matrix dimensions don't match coherence fingerprint dimension
    #[inline]
    pub fn apply_quantum_gate(&mut self, gate_matrix: &[f32]) -> Result<(), CognitiveError> {
        self.coherence_fingerprint.apply_gate(gate_matrix)
    }

    /// Measure entanglement with another quantum signature
    #[inline]
    #[must_use]
    pub fn measure_entanglement(&self, other: &Self) -> Option<f32> {
        self.coherence_fingerprint
            .entanglement_measure(&other.coherence_fingerprint)
    }

    /// Check if quantum signature has valid coherence
    #[inline]
    #[must_use]
    pub fn has_valid_coherence(&self) -> bool {
        !self.coherence_fingerprint.is_empty()
    }

    /// Create entanglement bond with another quantum signature
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError::LockPoisoned` if the entanglement bonds `RwLock` is poisoned
    #[inline]
    pub async fn create_entanglement_bond(
        &self,
        target_id: Uuid,
        bond_strength: f32,
        entanglement_type: EntanglementType,
    ) -> CognitiveResult<()> {
        let bond = EntanglementBond::new(target_id, bond_strength, entanglement_type);

        // Use write lock for interior mutability
        // Lock is held only during the push operation, then immediately released
        self.entanglement_bonds.write().await.push(bond);

        Ok(())
    }

    /// Get all entanglement bonds
    #[inline]
    pub async fn entanglement_bonds(&self) -> Vec<EntanglementBond> {
        // Read lock is held only during clone, then released
        self.entanglement_bonds.read().await.clone()
    }

    /// Get coherence fingerprint for quantum state access
    #[inline]
    #[must_use]
    pub fn coherence_fingerprint(&self) -> &AlignedCoherenceFingerprint {
        &self.coherence_fingerprint
    }

    /// Get superposition contexts for quantum routing
    #[inline]
    #[must_use]
    pub fn superposition_contexts(&self) -> &Vec<Arc<str>> {
        &self.superposition_contexts
    }

    /// Get creation timestamp for decoherence calculations
    #[inline]
    #[must_use]
    pub fn creation_time(&self) -> SystemTime {
        self.creation_time
    }

    /// Get decoherence rate for quantum state decay
    #[inline]
    #[must_use]
    pub fn decoherence_rate(&self) -> f64 {
        self.decoherence_rate
    }
}

impl Default for QuantumSignature {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Quantum entanglement bond between cognitive states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntanglementBond {
    /// Target entity ID
    pub target_id: Uuid,
    /// Bond strength (0.0 to 1.0)
    pub bond_strength: f32,
    /// Entanglement type classification
    pub entanglement_type: EntanglementType,
    /// Creation timestamp
    pub created_at: SystemTime,
}

impl EntanglementBond {
    /// Create new entanglement bond
    #[inline]
    #[must_use]
    #[allow(dead_code)] // TODO: Implement quantum entanglement bonds
    pub fn new(target_id: Uuid, bond_strength: f32, entanglement_type: EntanglementType) -> Self {
        Self {
            target_id,
            bond_strength: bond_strength.clamp(0.0, 1.0),
            entanglement_type,
            created_at: SystemTime::now(),
        }
    }
}

/// Types of quantum entanglement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EntanglementType {
    /// Semantic meaning entanglement
    Semantic = 0,
    /// Temporal sequence entanglement
    Temporal = 1,
    /// Causal relationship entanglement
    Causal = 2,
    /// Emergent pattern entanglement
    Emergent = 3,
    /// Werner state entanglement
    Werner = 4,
    /// Weak entanglement
    Weak = 5,
    /// Bell state entanglement
    Bell = 6,
    /// Bell pair entanglement
    BellPair = 7,
}
