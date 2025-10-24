//! Hybrid retrieval strategy combining multiple approaches

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::stream::StreamExt;
use tokio::sync::oneshot;

use crate::domain::memory::cognitive::types::{
    CognitiveProcessor, CognitiveProcessorConfig, DecisionOutcome,
};
use crate::memory::filter::MemoryFilter;
use crate::memory::utils::Result;
use crate::memory::vector::VectorStore;

use super::strategy::RetrievalStrategy;
use super::types::{PendingRetrieval, RetrievalMethod, RetrievalResult};

/// Hybrid retrieval strategy combining multiple approaches
pub struct HybridRetrieval<V: VectorStore> {
    vector_store: V,
    strategies: Arc<Vec<Arc<dyn RetrievalStrategy>>>,
    weights: Arc<HashMap<String, f32>>,
    cognitive_processor: Arc<CognitiveProcessor>,
}

impl<V: VectorStore> HybridRetrieval<V> {
    /// Create a new hybrid retrieval strategy
    pub fn new(vector_store: V) -> Self {
        let mut weights = HashMap::new();
        weights.insert("semantic".to_string(), 0.6);
        weights.insert("keyword".to_string(), 0.2);
        weights.insert("temporal".to_string(), 0.2);

        // Initialize cognitive processor with 0.7 decision threshold
        let processor_config = CognitiveProcessorConfig {
            batch_size: 32,
            decision_threshold: 0.7,
            learning_rate: 0.01,
            max_iterations: 1000,
        };

        Self {
            vector_store,
            strategies: Arc::new(Vec::new()),
            weights: Arc::new(weights),
            cognitive_processor: Arc::new(CognitiveProcessor::new(processor_config)),
        }
    }

    /// Add a retrieval strategy
    pub fn add_strategy(mut self, strategy: Arc<dyn RetrievalStrategy>) -> Self {
        Arc::make_mut(&mut self.strategies).push(strategy);
        self
    }

    /// Set weight for a strategy
    pub fn set_weight(mut self, strategy_name: &str, weight: f32) -> Self {
        Arc::make_mut(&mut self.weights).insert(strategy_name.to_string(), weight);
        self
    }

    /// Get vector similarity results from the vector store
    pub async fn get_vector_similarity(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<RetrievalResult>> {
        let filter = crate::memory::filter::MemoryFilter::new();

        // COGNITIVE PROCESSING: Evaluate query embedding for confidence
        // This determines if the query itself represents valid search intent
        let query_decision = self.cognitive_processor.process(&query_vector);

        let search_stream = self.vector_store.search(query_vector, limit, Some(filter));

        // Collect all results from the stream
        let results: Vec<_> = search_stream.collect().await;

        // Apply cognitive decision to filter and enhance results
        let retrieval_results: Vec<RetrievalResult> = match query_decision {
            Ok(decision) => {
                match decision.outcome {
                    DecisionOutcome::Accept => {
                        // Include results with cognitive confidence weighting
                        log::debug!(
                            "HybridRetrieval: Query ACCEPTED with confidence {:.4}",
                            decision.confidence
                        );

                        results
                            .into_iter()
                            .map(|result| {
                                let mut metadata = HashMap::new();
                                metadata.insert(
                                    "cognitive_confidence".to_string(),
                                    serde_json::json!(decision.confidence),
                                );
                                metadata.insert(
                                    "cognitive_outcome".to_string(),
                                    serde_json::json!("Accept"),
                                );

                                // Weight similarity by cognitive confidence
                                let weighted_score = result.score * decision.confidence;

                                RetrievalResult {
                                    id: result.id,
                                    method: RetrievalMethod::VectorSimilarity,
                                    score: weighted_score,
                                    metadata,
                                }
                            })
                            .collect()
                    }
                    DecisionOutcome::Defer => {
                        // Include results with reduced confidence
                        log::debug!(
                            "HybridRetrieval: Query DEFERRED with confidence {:.4}",
                            decision.confidence
                        );

                        results
                            .into_iter()
                            .map(|result| {
                                let mut metadata = HashMap::new();
                                metadata.insert(
                                    "cognitive_confidence".to_string(),
                                    serde_json::json!(decision.confidence),
                                );
                                metadata.insert(
                                    "cognitive_outcome".to_string(),
                                    serde_json::json!("Defer"),
                                );

                                // Reduce score for deferred queries
                                let reduced_score = result.score * 0.5;

                                RetrievalResult {
                                    id: result.id,
                                    method: RetrievalMethod::VectorSimilarity,
                                    score: reduced_score,
                                    metadata,
                                }
                            })
                            .collect()
                    }
                    DecisionOutcome::Reject | DecisionOutcome::RequestInfo => {
                        // Filter out all results for rejected/uncertain queries
                        log::debug!(
                            "HybridRetrieval: Query {:?} with confidence {:.4} - filtering all results",
                            decision.outcome,
                            decision.confidence
                        );
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                // Fallback on cognitive processor error - return unfiltered results
                log::warn!(
                    "HybridRetrieval: Cognitive processor error: {} - using unfiltered results",
                    e
                );

                results
                    .into_iter()
                    .map(|result| RetrievalResult {
                        id: result.id,
                        method: RetrievalMethod::VectorSimilarity,
                        score: result.score,
                        metadata: HashMap::new(),
                    })
                    .collect()
            }
        };

        Ok(retrieval_results)
    }
}

impl<V: VectorStore + Send + Sync + 'static> RetrievalStrategy for HybridRetrieval<V> {
    fn retrieve(
        &self,
        query: String,
        limit: usize,
        filter: Option<MemoryFilter>,
    ) -> PendingRetrieval {
        let (tx, rx) = oneshot::channel();
        let strategies = self.strategies.clone();
        let weights = self.weights.clone();

        tokio::spawn(async move {
            let result: Result<Vec<RetrievalResult>> = (async {
                let mut all_results: HashMap<String, (f32, RetrievalResult)> = HashMap::new();

                // Get results from each strategy
                for strategy in &*strategies {
                    let results = strategy
                        .retrieve(query.clone(), limit * 2, filter.clone())
                        .await?;
                    let weight = weights.get(strategy.name()).unwrap_or(&1.0);

                    for result in results {
                        let weighted_score = result.score * weight;

                        all_results
                            .entry(result.id.clone())
                            .and_modify(|(score, _)| *score += weighted_score)
                            .or_insert((weighted_score, result));
                    }
                }

                // Sort by combined score and take top results
                let mut sorted_results: Vec<_> = all_results
                    .into_iter()
                    .map(|(_, (score, mut result))| {
                        result.score = score;
                        result
                    })
                    .collect();

                sorted_results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted_results.truncate(limit);

                Ok(sorted_results)
            })
            .await;

            let _ = tx.send(result);
        });

        PendingRetrieval::new(rx)
    }

    fn name(&self) -> &str {
        "hybrid"
    }
}
