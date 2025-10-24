//! Quantum-inspired router for local cognitive operations

use thiserror::Error;

use super::types::{EnhancedQuery, RoutingDecision, RoutingStrategy};
use crate::domain::memory::cognitive::types::CognitiveState;

/// Quantum router error types
#[derive(Error, Debug)]
pub enum QuantumRouterError {
    #[error("Superposition error: {0}")]
    SuperpositionError(String),

    #[error("Entanglement error: {0}")]
    EntanglementError(String),

    #[error("Measurement error: {0}")]
    MeasurementError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Local quantum router
#[derive(Debug, Clone)]
pub struct QuantumRouter {
    pub routing_strategy: RoutingStrategy,
    pub coherence_threshold: f64,
}

impl QuantumRouter {
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            routing_strategy: strategy,
            coherence_threshold: 0.7,
        }
    }

    pub async fn route(
        &self,
        query: EnhancedQuery,
        cognitive_state: Option<&CognitiveState>,
    ) -> Result<RoutingDecision, QuantumRouterError> {
        // ═══════════════════════════════════════════════════════════════════
        // Extract quantum metrics from cognitive state
        // ═══════════════════════════════════════════════════════════════════
        let (coherence, entropy, collapse_prob) = if let Some(state) = cognitive_state {
            let coherence = state.coherence_state_probability();
            let entropy = state.quantum_entropy();
            let collapse_prob = state.quantum_collapse_probability();

            // Validate quantum metrics for NaN/Infinity
            if !coherence.is_finite() || !entropy.is_finite() || !collapse_prob.is_finite() {
                log::error!(
                    "Invalid quantum metrics detected - coherence: {}, entropy: {}, collapse: {}",
                    coherence,
                    entropy,
                    collapse_prob
                );
                return Err(QuantumRouterError::MeasurementError(format!(
                    "Invalid quantum state metrics (NaN or Infinity): coherence={}, entropy={}, collapse={}",
                    coherence, entropy, collapse_prob
                )));
            }

            log::debug!(
                "Quantum routing metrics - coherence: {:.4}, entropy: {:.4}, collapse: {:.4}",
                coherence,
                entropy,
                collapse_prob
            );

            (coherence, entropy, collapse_prob)
        } else {
            // No cognitive state - use neutral values for perfect conditions
            // coherence=1.0 (perfect), entropy=0.0 (no uncertainty), collapse=0.0 (stable)
            log::debug!("No cognitive state provided, using neutral quantum metrics");
            (1.0, 0.0, 0.0)
        };

        // Analyze query characteristics to determine optimal strategy
        let query_text = query.query.to_lowercase();

        // Determine strategy based on query patterns
        let (strategy, confidence, reasoning) = if query_text.contains("recent")
            || query_text.contains("latest")
            || query_text.contains("today")
            || query_text.contains("yesterday")
            || query_text.contains("last week")
        {
            // Temporal queries - use Causal strategy for time-ordered results
            (
                RoutingStrategy::Causal,
                0.9,
                "Query contains temporal keywords - using time-ordered search",
            )
        } else if query_text.contains("similar to")
            || query_text.contains("like")
            || query_text.contains("related")
            || query_text.contains("about")
        {
            // Semantic similarity queries - use Quantum (vector) strategy
            (
                RoutingStrategy::Quantum,
                0.85,
                "Query seeks semantic similarity - using vector search",
            )
        } else if query_text.contains("*")
            || query_text.contains("?")
            || query_text.contains("%")
            || query_text.starts_with("pattern:")
        {
            // Pattern matching queries - use Emergent strategy
            (
                RoutingStrategy::Emergent,
                0.8,
                "Query contains wildcards or pattern indicators",
            )
        } else if query_text.split_whitespace().count() <= 3 {
            // Short keyword queries - use Attention (content) strategy
            (
                RoutingStrategy::Attention,
                0.75,
                "Short query - using keyword-based content search",
            )
        } else {
            // Default: analyze query complexity for hybrid approach
            let word_count = query_text.split_whitespace().count();

            if word_count > 10 {
                // Complex queries benefit from multiple strategies
                (
                    RoutingStrategy::Hybrid(vec![
                        RoutingStrategy::Quantum,
                        RoutingStrategy::Attention,
                    ]),
                    0.7,
                    "Complex query - using hybrid approach",
                )
            } else {
                // Medium-length queries: semantic search works well
                (
                    RoutingStrategy::Quantum,
                    0.8,
                    "Standard query - using semantic vector search",
                )
            }
        };

        // ═══════════════════════════════════════════════════════════════════
        // Apply quantum adjustments to base confidence
        // ═══════════════════════════════════════════════════════════════════
        let base_confidence = confidence; // Save original for logging

        let adjusted_confidence = if cognitive_state.is_some() {
            // Apply quantum corrections to base confidence
            // Formula: base_confidence × coherence_factor × entropy_penalty

            // Coherence factor: higher coherence = higher confidence
            let coherence_factor = coherence as f64;

            // Entropy penalty: higher entropy = lower confidence
            // Use 1/(1+entropy) to map [0,∞) → (0,1]
            // Examples:
            //   entropy=0.0 → penalty=1.0 (no reduction)
            //   entropy=1.0 → penalty=0.5 (50% reduction)
            //   entropy=2.0 → penalty=0.33 (67% reduction)
            let entropy_penalty = 1.0 / (1.0 + entropy);

            let quantum_adjusted = base_confidence * coherence_factor * entropy_penalty;

            // Clamp to valid confidence range [0.0, 1.0]
            quantum_adjusted.clamp(0.0, 1.0)
        } else {
            // No cognitive state - use base confidence unchanged
            base_confidence
        };

        log::debug!(
            "Routing confidence adjustment: base={:.4}, quantum_adjusted={:.4} (coherence={:.4}, entropy_penalty={:.4})",
            base_confidence,
            adjusted_confidence,
            coherence as f64,
            if cognitive_state.is_some() {
                1.0 / (1.0 + entropy)
            } else {
                1.0
            }
        );

        // Check if override strategy was requested in enhanced query
        // Empty Hybrid means "let router decide", any other strategy is an override
        let initial_strategy = match query.routing_strategy {
            RoutingStrategy::Hybrid(ref strategies) if strategies.is_empty() => {
                // Empty Hybrid - use query-pattern-determined strategy
                strategy
            }
            _ => {
                // Any specific strategy (including non-empty Hybrid) overrides pattern detection
                query.routing_strategy
            }
        };

        // ═══════════════════════════════════════════════════════════════════
        // Check for quantum instability (high collapse probability)
        // ═══════════════════════════════════════════════════════════════════

        // Warn if quantum collapse probability is high (> 0.8)
        if collapse_prob > 0.8 {
            log::warn!(
                "High quantum collapse probability ({:.4}) - routing may be unstable for query: {:?}",
                collapse_prob,
                query.query.chars().take(50).collect::<String>()
            );
        }

        // Critical instability: force fallback strategy if collapse risk is extreme
        let final_strategy = if collapse_prob > 0.9 {
            log::warn!(
                "CRITICAL: Extreme quantum instability (collapse={:.4}) - forcing Attention fallback strategy",
                collapse_prob
            );

            // Force safe fallback strategy for critical instability
            // Attention (keyword search) is most deterministic and predictable
            RoutingStrategy::Attention
        } else {
            // Use strategy after query override consideration
            initial_strategy
        };

        // ═══════════════════════════════════════════════════════════════════
        // Construct RoutingDecision with quantum reasoning
        // ═══════════════════════════════════════════════════════════════════
        Ok(RoutingDecision {
            strategy: final_strategy,
            confidence: adjusted_confidence,
            reasoning: if cognitive_state.is_some() {
                format!(
                    "{} (quantum: coherence={:.2}, entropy={:.2}, collapse={:.2})",
                    reasoning, coherence, entropy, collapse_prob
                )
            } else {
                reasoning.to_string()
            },
        })
    }
}

impl Default for QuantumRouter {
    fn default() -> Self {
        Self::new(RoutingStrategy::Attention)
    }
}
