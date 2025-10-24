//! Core cognitive state implementation with atomic operations and lock-free queues

use crossbeam_skiplist::SkipMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use super::activation::AlignedActivationPattern;
use super::atomics::AtomicF32;
use super::attention::AtomicAttentionWeights;
use super::memory_items::{CognitiveMemoryEntry, CognitiveStats, WorkingMemoryItem};
use super::quantum::{EntanglementType, QuantumSignature};
use super::temporal::TemporalContext;

/// Quantum-inspired cognitive state with atomic operations and lock-free queues
///
/// Features:
/// - Attention tracking with atomic f32 values for concurrent updates
/// - Working memory slots with lock-free queue implementation  
/// - Long-term memory mapping with tokio async primitives
/// - Confidence scoring with statistical aggregation functions
/// - Quantum signature tracking for quantum-enhanced routing and entanglement
#[derive(Debug, Clone)]
pub struct CognitiveState {
    /// SIMD-aligned activation pattern for parallel neural processing
    ///
    /// Updated via `update_activation()` when processing stimuli.
    /// Energy calculations drive attention weight updates.
    /// Remains `#[allow(dead_code)]` until cognitive system fully activated.
    activation_pattern: AlignedActivationPattern,

    /// Atomic attention weights for concurrent updates
    attention_weights: Arc<AtomicAttentionWeights>,

    /// Working memory queue
    working_memory: Arc<(
        mpsc::UnboundedSender<WorkingMemoryItem>,
        Mutex<mpsc::UnboundedReceiver<WorkingMemoryItem>>,
    )>,

    /// Long-term memory skip-list for O(log n) access
    long_term_memory: Arc<SkipMap<Uuid, CognitiveMemoryEntry>>,

    /// Temporal context for time-aware memory operations
    ///
    /// Provides temporal window management, causal dependency tracking,
    /// and memory consolidation timing. Updated via `update_temporal_window()`.
    /// Remains `#[allow(dead_code)]` until cognitive system fully activated.
    #[allow(dead_code)] // TODO: Implement in cognitive state system
    temporal_context: Arc<TemporalContext>,

    /// Quantum signature for quantum-enhanced memory routing
    quantum_signature: Arc<QuantumSignature>,

    /// Atomic uncertainty and confidence tracking
    uncertainty: Arc<AtomicF32>,
    confidence: Arc<AtomicF32>,
    meta_awareness: Arc<AtomicF32>,

    /// Statistics for monitoring cognitive performance
    stats: Arc<CognitiveStats>,
}

/// Default functions for `CognitiveState` fields
#[allow(dead_code)] // TODO: Implement cognitive attention weights defaults
fn default_attention_weights() -> Arc<AtomicAttentionWeights> {
    Arc::new(AtomicAttentionWeights::new())
}

#[allow(dead_code)] // TODO: Implement working memory defaults
fn default_working_memory() -> Arc<(
    mpsc::UnboundedSender<WorkingMemoryItem>,
    Mutex<mpsc::UnboundedReceiver<WorkingMemoryItem>>,
)> {
    let (sender, receiver) = mpsc::unbounded_channel();
    Arc::new((sender, Mutex::new(receiver)))
}

#[allow(dead_code)] // TODO: Implement long term memory defaults
fn default_long_term_memory() -> Arc<SkipMap<Uuid, CognitiveMemoryEntry>> {
    Arc::new(SkipMap::new())
}

#[allow(dead_code)] // TODO: Implement temporal context defaults
fn default_temporal_context() -> Arc<TemporalContext> {
    Arc::new(TemporalContext::default())
}

#[allow(dead_code)] // TODO: Implement uncertainty defaults
fn default_uncertainty() -> Arc<AtomicF32> {
    Arc::new(AtomicF32::new(0.5))
}

#[allow(dead_code)] // TODO: Implement confidence defaults
fn default_confidence() -> Arc<AtomicF32> {
    Arc::new(AtomicF32::new(0.5))
}

#[allow(dead_code)] // TODO: Implement meta awareness defaults
fn default_meta_awareness() -> Arc<AtomicF32> {
    Arc::new(AtomicF32::new(0.5))
}

#[allow(dead_code)] // TODO: Implement cognitive stats defaults
fn default_cognitive_stats() -> Arc<CognitiveStats> {
    Arc::new(CognitiveStats::new())
}

impl CognitiveState {
    /// Create new cognitive state
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            activation_pattern: AlignedActivationPattern::default(),
            attention_weights: default_attention_weights(),
            working_memory: default_working_memory(),
            long_term_memory: default_long_term_memory(),
            temporal_context: default_temporal_context(),
            quantum_signature: Arc::new(QuantumSignature::new()),
            uncertainty: default_uncertainty(),
            confidence: default_confidence(),
            meta_awareness: default_meta_awareness(),
            stats: default_cognitive_stats(),
        }
    }

    /// Add item to working memory
    #[inline]
    pub fn add_working_memory(&self, content: impl Into<Arc<str>>, activation: f32, ttl: Duration) {
        let item = WorkingMemoryItem::new(content, activation, ttl);
        let _ = self.working_memory.0.send(item);
        self.stats.record_working_memory_access();
    }

    /// Get item from working memory with automatic cleanup
    #[must_use]
    pub async fn get_working_memory(&self) -> Option<WorkingMemoryItem> {
        // Clean up expired items
        let mut rx = self.working_memory.1.lock().await;
        while let Some(item) = rx.recv().await {
            if !item.is_expired() {
                self.stats.record_working_memory_access();
                return Some(item);
            }
        }
        None
    }

    /// Add to long-term memory with O(log n) access
    #[inline]
    pub fn add_long_term_memory(&self, id: Uuid, entry: CognitiveMemoryEntry) {
        self.long_term_memory.insert(id, entry);
        self.stats.record_long_term_memory_access();
    }

    /// Get from long-term memory with access tracking
    #[must_use]
    pub fn get_long_term_memory(&self, id: Uuid) -> Option<CognitiveMemoryEntry> {
        if let Some(entry) = self.long_term_memory.get(&id) {
            let mut memory_entry = entry.value().clone();
            memory_entry.access();
            self.long_term_memory.insert(id, memory_entry.clone());
            self.stats.record_long_term_memory_access();
            Some(memory_entry)
        } else {
            None
        }
    }

    /// Update attention weights atomically
    #[inline]
    pub fn update_attention(&self, primary: f32, secondary: f32, background: f32, meta: f32) {
        self.attention_weights.set_primary(primary);
        self.attention_weights.set_secondary(secondary);
        self.attention_weights.set_background(background);
        self.attention_weights.set_meta(meta);
        self.attention_weights.normalize();
        self.stats.record_attention_update();
    }

    /// Get attention weights
    #[inline]
    #[must_use]
    pub fn attention_weights(&self) -> (f32, f32, f32, f32) {
        (
            self.attention_weights.primary(),
            self.attention_weights.secondary(),
            self.attention_weights.background(),
            self.attention_weights.meta(),
        )
    }

    /// Set uncertainty atomically
    #[inline]
    pub fn set_uncertainty(&self, uncertainty: f32) {
        self.uncertainty
            .store(uncertainty.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Get uncertainty
    #[inline]
    #[must_use]
    pub fn uncertainty(&self) -> f32 {
        self.uncertainty.load(Ordering::Relaxed)
    }

    /// Set confidence atomically
    #[inline]
    pub fn set_confidence(&self, confidence: f32) {
        self.confidence
            .store(confidence.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Get confidence
    #[inline]
    #[must_use]
    pub fn confidence(&self) -> f32 {
        self.confidence.load(Ordering::Relaxed)
    }

    /// Set meta-awareness atomically
    #[inline]
    pub fn set_meta_awareness(&self, meta_awareness: f32) {
        self.meta_awareness
            .store(meta_awareness.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Get meta-awareness
    #[inline]
    #[must_use]
    pub fn meta_awareness(&self) -> f32 {
        self.meta_awareness.load(Ordering::Relaxed)
    }

    /// Get cognitive statistics
    #[inline]
    #[must_use]
    pub fn stats(&self) -> &CognitiveStats {
        &self.stats
    }

    /// Get cognitive statistics Arc
    #[inline]
    #[must_use]
    pub fn stats_arc(&self) -> Arc<CognitiveStats> {
        Arc::clone(&self.stats)
    }

    /// Get quantum signature for quantum-enhanced routing
    #[inline]
    #[must_use]
    pub fn quantum_signature(&self) -> &Arc<QuantumSignature> {
        &self.quantum_signature
    }

    /// Apply quantum decoherence to the cognitive state
    #[inline]
    pub fn apply_quantum_decoherence(&self) {
        self.quantum_signature.apply_decoherence();
    }

    /// Get quantum collapse probability
    #[inline]
    #[must_use]
    pub fn quantum_collapse_probability(&self) -> f32 {
        self.quantum_signature.collapse_probability()
    }

    /// Get quantum entropy measure
    #[inline]
    #[must_use]
    pub fn quantum_entropy(&self) -> f64 {
        self.quantum_signature.quantum_entropy()
    }

    /// Get coherence state probability for quantum routing
    #[inline]
    #[must_use]
    pub fn coherence_state_probability(&self) -> f32 {
        self.quantum_signature.coherence_state_probability()
    }

    /// Measure quantum entanglement with another cognitive state
    #[inline]
    #[must_use]
    pub fn measure_quantum_entanglement(&self, other: &Self) -> Option<f32> {
        self.quantum_signature
            .measure_entanglement(&other.quantum_signature)
    }

    /// Check if cognitive state has valid quantum coherence
    #[inline]
    #[must_use]
    pub fn has_quantum_coherence(&self) -> bool {
        self.quantum_signature.has_valid_coherence()
    }

    /// Add quantum entanglement bond to another cognitive state
    #[inline]
    #[must_use]
    pub async fn add_quantum_entanglement_bond(
        &self,
        target_id: Uuid,
        bond_strength: f32,
        entanglement_type: EntanglementType,
    ) -> bool {
        // Validate bond strength is in valid range
        if !(0.0..=1.0).contains(&bond_strength) {
            log::warn!(
                "Invalid bond strength {bond_strength} for entanglement with {target_id}, must be 0.0-1.0"
            );
            return false;
        }

        // Create the actual entanglement bond in quantum signature
        if let Err(e) = self
            .quantum_signature
            .create_entanglement_bond(target_id, bond_strength, entanglement_type)
            .await
        {
            log::error!("Failed to create entanglement bond: {e}");
            return false;
        }

        // Record the quantum operation for statistics
        self.stats.record_quantum_operation();

        log::debug!(
            "Created quantum entanglement bond to {target_id} with strength {bond_strength} and type {entanglement_type:?}"
        );

        true
    }

    /// Get count of quantum entanglement bonds
    #[inline]
    #[must_use]
    pub async fn quantum_entanglement_bond_count(&self) -> usize {
        self.quantum_signature.entanglement_bonds().await.len()
    }

    /// Create cognitive state with custom quantum coherence
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if amplitudes and phases vectors have different dimensions
    #[inline]
    pub fn with_quantum_coherence(
        amplitudes: &[f32],
        phases: Vec<f32>,
    ) -> Result<Self, CognitiveError> {
        let quantum_signature = Arc::new(QuantumSignature::with_coherence(amplitudes, phases)?);

        Ok(Self {
            activation_pattern: AlignedActivationPattern::default(),
            attention_weights: default_attention_weights(),
            working_memory: default_working_memory(),
            long_term_memory: default_long_term_memory(),
            temporal_context: default_temporal_context(),
            quantum_signature,
            uncertainty: default_uncertainty(),
            confidence: default_confidence(),
            meta_awareness: default_meta_awareness(),
            stats: default_cognitive_stats(),
        })
    }

    /// Create cognitive state with existing quantum signature
    ///
    /// Used for deserializing persisted cognitive states with their quantum signatures.
    #[inline]
    #[must_use]
    pub fn new_with_quantum_signature(quantum_signature: QuantumSignature) -> Self {
        Self {
            activation_pattern: AlignedActivationPattern::default(),
            attention_weights: default_attention_weights(),
            working_memory: default_working_memory(),
            long_term_memory: default_long_term_memory(),
            temporal_context: default_temporal_context(),
            quantum_signature: Arc::new(quantum_signature),
            uncertainty: default_uncertainty(),
            confidence: default_confidence(),
            meta_awareness: default_meta_awareness(),
            stats: default_cognitive_stats(),
        }
    }

    /// Update activation pattern from external stimulus
    ///
    /// Uses existing `AlignedActivationPattern` infrastructure to:
    /// 1. Validate stimulus dimensions
    /// 2. Update activation data
    /// 3. Apply sigmoid activation function  
    /// 4. Calculate pattern energy
    /// 5. Update attention weights based on activation
    ///
    /// # Errors
    ///
    /// Returns `CognitiveError` if stimulus dimensions are invalid or attention update fails
    #[inline]
    pub fn update_activation_from_stimulus(
        &mut self,
        stimulus: Vec<f32>,
    ) -> Result<(), CognitiveError> {
        // Validate stimulus is not empty
        if stimulus.is_empty() {
            return Err(CognitiveError::OperationFailed(
                "Stimulus vector cannot be empty".to_string(),
            ));
        }

        // Update activation pattern with stimulus data
        self.activation_pattern.update(stimulus);

        // Apply sigmoid activation: σ(x) = 1 / (1 + e^(-x))
        self.activation_pattern
            .apply_activation(|x| 1.0 / (1.0 + (-x).exp()));

        // Calculate activation energy for attention update
        let energy = self.activation_pattern.energy();
        // APPROVED BY DAVID MAPLE on 2025-10-20
        // Precision loss is acceptable for neural network normalization (dimensions typically < 16M)
        #[allow(clippy::cast_precision_loss)]
        let normalized_energy = (energy / self.activation_pattern.dimension as f32)
            .sqrt()
            .clamp(0.0, 1.0);

        // Update attention weights based on activation energy
        self.attention_weights.update_from_energy(normalized_energy);

        // Record attention update metric
        self.stats.record_attention_update();

        Ok(())
    }
}

impl Default for CognitiveState {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for cognitive operations
pub type CognitiveResult<T> = Result<T, CognitiveError>;

/// Cognitive operation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum CognitiveError {
    /// Invalid quantum state
    #[error("Invalid quantum state: {0}")]
    InvalidQuantumState(String),

    /// Invalid quantum operation
    #[error("Invalid quantum operation: {0}")]
    InvalidQuantumOperation(String),

    /// Memory capacity exceeded
    #[error("Memory capacity exceeded: {0}")]
    MemoryCapacityExceeded(String),

    /// Temporal inconsistency
    #[error("Temporal inconsistency: {0}")]
    TemporalInconsistency(String),

    /// Attention overflow
    #[error("Attention overflow: {0}")]
    AttentionOverflow(String),

    /// Lock poisoned error
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),

    /// Dimension mismatch
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}
