//! Cognitive filtering and state management

use super::types::{DeferredResult, FinalResult};

/// State for multi-stage cognitive filtering
pub(crate) struct CognitiveSearchState {
    /// Results deferred for secondary evaluation with confidence scores
    pub(crate) deferred_results: Vec<DeferredResult>,

    /// Final accepted results
    pub(crate) final_results: Vec<FinalResult>,
}

impl CognitiveSearchState {
    pub(crate) fn new() -> Self {
        Self {
            deferred_results: Vec::new(),
            final_results: Vec::new(),
        }
    }
}

/// Process deferred results with secondary threshold evaluation
///
/// Results with confidence above the secondary threshold are promoted to final results.
/// This implements a two-stage filtering approach for medium-confidence items.
///
/// # Arguments
/// * `state` - Mutable reference to cognitive search state
/// * `threshold` - Secondary threshold for deferred result acceptance (typically 0.56 = 0.7 * 0.8)
pub(crate) fn process_deferred_results(state: &mut CognitiveSearchState, threshold: f32) {
    state
        .deferred_results
        .retain(|(id, vector, similarity, metadata, confidence)| {
            if *confidence >= threshold {
                log::debug!(
                    "Promoting deferred result: id={}, confidence={:.4}, threshold={:.4}",
                    id,
                    confidence,
                    threshold
                );
                state.final_results.push((
                    id.clone(),
                    vector.clone(),
                    *similarity,
                    metadata.clone(),
                ));
                false // Remove from deferred queue
            } else {
                log::trace!(
                    "Rejecting deferred result: id={}, confidence={:.4}, threshold={:.4}",
                    id,
                    confidence,
                    threshold
                );
                false // Remove from deferred queue (rejected)
            }
        });
}
