//! Main VectorSearch implementation

use std::cmp::Ordering;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::capability::registry::TextEmbeddingModel;
use crate::capability::traits::TextEmbeddingCapable;
use crate::domain::memory::cognitive::types::{
    CognitiveProcessor, CognitiveProcessorConfig, DecisionOutcome,
};
use crate::memory::constants::SEARCH_TASK;
use crate::memory::utils::error::Result;
use crate::memory::vector::vector_store::VectorStore;

use super::cognitive::{CognitiveSearchState, process_deferred_results};
use super::helpers::task_string;
use super::options::SearchOptions;
use super::types::SearchResult;

/// High-performance vector search implementation
///
/// This implementation provides blazing-fast vector search using:
/// - SIMD-optimized similarity calculations
/// - Memory-efficient result processing
/// - Thread-safe operations for concurrent access
/// - Comprehensive filtering and ranking capabilities
#[derive(Debug, Clone)]
pub struct VectorSearch {
    /// Vector store for persistence and retrieval
    store: Arc<dyn VectorStore>,
    /// Embedding model for text-to-vector conversion
    embedding_model: TextEmbeddingModel,
    /// Default search options
    default_options: SearchOptions,
    /// Cognitive processor for intelligent result filtering and adaptive thresholds
    cognitive_processor: Arc<CognitiveProcessor>,
}

impl VectorSearch {
    /// Create a new VectorSearch with default options
    ///
    /// # Arguments
    /// * `store` - Vector store implementation
    /// * `embedding_model` - Embedding model for text processing
    ///
    /// # Returns
    /// New VectorSearch instance
    pub fn new(store: Arc<dyn VectorStore>, embedding_model: TextEmbeddingModel) -> Self {
        let processor_config = CognitiveProcessorConfig {
            batch_size: 32,
            decision_threshold: 0.7, // Matches SearchOptions::default() min_similarity
            learning_rate: 0.01,
            max_iterations: 1000,
        };

        Self {
            store,
            embedding_model,
            default_options: SearchOptions::default(),
            cognitive_processor: Arc::new(CognitiveProcessor::new(processor_config)),
        }
    }

    /// Create with custom default options
    pub fn with_options(
        store: Arc<dyn VectorStore>,
        embedding_model: TextEmbeddingModel,
        default_options: SearchOptions,
    ) -> Self {
        // Extract threshold from options for processor config (adaptive threshold principle)
        let decision_threshold = default_options.min_similarity.unwrap_or(0.7);
        let processor_config = CognitiveProcessorConfig {
            batch_size: 32,
            decision_threshold,
            learning_rate: 0.01,
            max_iterations: 1000,
        };

        Self {
            store,
            embedding_model,
            default_options,
            cognitive_processor: Arc::new(CognitiveProcessor::new(processor_config)),
        }
    }

    /// Search by text query (synchronous)
    ///
    /// # Arguments
    /// * `text` - Text query to search for
    /// * `options` - Search options (uses defaults if None)
    ///
    /// # Returns
    /// Result containing ranked search results
    pub async fn search_by_text(
        &self,
        text: &str,
        options: Option<SearchOptions>,
    ) -> Result<Vec<SearchResult>> {
        // Validate input
        if text.is_empty() {
            return Ok(Vec::new());
        }

        // Generate embedding for the text (asynchronous)
        let embedding = self
            .embedding_model
            .embed(text, task_string(SEARCH_TASK))
            .await?;

        // Search by embedding
        self.search_by_embedding(&embedding, options)
    }

    /// Search by embedding vector (synchronous)
    ///
    /// # Arguments
    /// * `embedding` - Query vector
    /// * `options` - Search options (uses defaults if None)
    ///
    /// # Returns
    /// Result containing ranked search results
    pub fn search_by_embedding(
        &self,
        embedding: &[f32],
        options: Option<SearchOptions>,
    ) -> Result<Vec<SearchResult>> {
        let options = options.unwrap_or_else(|| self.default_options.clone());
        let options = options.validate()?;

        // Validate embedding
        if embedding.is_empty() {
            return Err(crate::memory::utils::error::Error::InvalidInput(
                "Embedding vector cannot be empty".to_string(),
            ));
        }

        if embedding.iter().any(|&x| !x.is_finite()) {
            return Err(crate::memory::utils::error::Error::InvalidInput(
                "Embedding vector contains NaN or infinite values".to_string(),
            ));
        }

        // Prepare search parameters
        let limit = options.candidate_limit.or(options.limit);
        let filters = options.filters.clone();

        // Search in vector store (synchronous)
        let results = self.store.search(embedding, limit, filters)?;

        // Use cognitive processor for intelligent result filtering with defer queue
        let filtered_results = if options.min_similarity.is_some() {
            let mut state = CognitiveSearchState::new();

            // Stage 1: Initial cognitive filtering with defer queue
            for (id, vector, similarity, metadata) in results {
                match self.cognitive_processor.process(&vector) {
                    Ok(decision) => {
                        match decision.outcome {
                            DecisionOutcome::Accept => {
                                log::debug!(
                                    "CognitiveProcessor ACCEPT: similarity={:.4}, confidence={:.4}",
                                    similarity,
                                    decision.confidence
                                );
                                state.final_results.push((id, vector, similarity, metadata));
                            }
                            DecisionOutcome::Defer => {
                                log::debug!(
                                    "CognitiveProcessor DEFER: similarity={:.4}, confidence={:.4}",
                                    similarity,
                                    decision.confidence
                                );
                                state.deferred_results.push((
                                    id,
                                    vector,
                                    similarity,
                                    metadata,
                                    decision.confidence,
                                ));
                            }
                            DecisionOutcome::Reject => {
                                log::trace!(
                                    "CognitiveProcessor REJECT: similarity={:.4}, confidence={:.4}",
                                    similarity,
                                    decision.confidence
                                );
                                // Excluded
                            }
                            DecisionOutcome::RequestInfo => {
                                log::debug!(
                                    "CognitiveProcessor REQUEST_INFO: similarity={:.4}",
                                    similarity
                                );
                                if let Some(ref callback) = options.request_info_callback {
                                    let should_accept =
                                        callback(&id, similarity, decision.confidence);
                                    if should_accept {
                                        log::debug!(
                                            "RequestInfo callback accepted: id={}, similarity={:.4}",
                                            id,
                                            similarity
                                        );
                                        state
                                            .final_results
                                            .push((id, vector, similarity, metadata));
                                    } else {
                                        log::debug!(
                                            "RequestInfo callback rejected: id={}, similarity={:.4}",
                                            id,
                                            similarity
                                        );
                                        // Rejected by callback - exclude from results
                                    }
                                } else {
                                    // Fallback: treat as deferred
                                    log::trace!(
                                        "No RequestInfo callback provided, treating as deferred: id={}",
                                        id
                                    );
                                    state.deferred_results.push((
                                        id,
                                        vector,
                                        similarity,
                                        metadata,
                                        decision.confidence,
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Fallback to similarity threshold on processor error
                        log::warn!("CognitiveProcessor error, using fallback: {}", e);
                        if similarity >= options.min_similarity.unwrap_or(0.7) {
                            state.final_results.push((id, vector, similarity, metadata));
                        }
                    }
                }
            }

            // Stage 2: Process deferred results with relaxed threshold
            let defer_threshold = options.min_similarity.unwrap_or(0.7) * 0.8; // 80% of main threshold
            process_deferred_results(&mut state, defer_threshold);

            state.final_results
        } else {
            results
        };

        // Convert to SearchResult format with comprehensive metadata
        let mut search_results = filtered_results
            .into_iter()
            .enumerate()
            .map(|(index, (id, vector, similarity, metadata))| {
                // Get decision confidence from processor
                let decision_confidence = self
                    .cognitive_processor
                    .process(&vector)
                    .ok()
                    .map(|decision| decision.confidence);

                let mut result = if let Some(meta) = metadata {
                    SearchResult::with_metadata(id, vector, similarity, meta)
                } else {
                    SearchResult::new(id, vector, similarity)
                };

                if let Some(confidence) = decision_confidence {
                    result = result.with_decision_confidence(confidence);
                }

                if options.include_rank.unwrap_or(false) {
                    result = result.with_rank(index + 1);
                }

                result
            })
            .collect::<Vec<_>>();

        // Apply minimum similarity threshold (post-cognitive filtering)
        if let Some(min_sim) = options.min_similarity {
            search_results.retain(|r| r.similarity >= min_sim);
        }

        // Sort by similarity (descending) with NaN handling
        search_results.sort_by(|a, b| {
            match (a.similarity.is_nan(), b.similarity.is_nan()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater, // NaN goes to end
                (false, true) => Ordering::Less,    // NaN goes to end
                (false, false) => b
                    .similarity
                    .partial_cmp(&a.similarity)
                    .unwrap_or(Ordering::Equal),
            }
        });

        // Apply final limit
        if let Some(limit) = options.limit
            && search_results.len() > limit
        {
            search_results.truncate(limit);
        }

        // Strip vectors if not requested (memory optimization)
        if !options.include_vectors.unwrap_or(false) {
            for result in &mut search_results {
                result.vector.clear();
            }
        }

        // Strip metadata if not requested
        if !options.include_metadata.unwrap_or(true) {
            for result in &mut search_results {
                result.metadata = None;
            }
        }

        Ok(search_results)
    }

    /// Batch search by multiple text queries
    ///
    /// # Arguments
    /// * `texts` - Array of text queries
    /// * `options` - Search options applied to all queries
    ///
    /// # Returns
    /// Result containing array of search results (one per query)
    pub async fn batch_search_by_text(
        &self,
        texts: &[String],
        options: Option<SearchOptions>,
    ) -> Result<Vec<Vec<SearchResult>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Create channel for results
        let (tx, mut rx) = mpsc::channel(texts.len());

        // Spawn concurrent search tasks
        for (index, text) in texts.iter().enumerate() {
            let searcher = self.clone();
            let text = text.clone();
            let opts = options.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                let result = searcher.search_by_text(&text, opts).await;
                if tx.send((index, result)).await.is_err() {
                    log::error!("Failed to send search result for batch index {}", index);
                }
            });
        }

        // Important: drop the original sender so the channel can close
        drop(tx);

        // Collect results in order
        let mut results = vec![Vec::new(); texts.len()];
        while let Some((index, result)) = rx.recv().await {
            match result {
                Ok(search_results) => results[index] = search_results,
                Err(e) => return Err(e),
            }
        }

        Ok(results)
    }

    /// Get the vector store reference
    pub fn store(&self) -> Arc<dyn VectorStore> {
        Arc::clone(&self.store)
    }

    /// Get the embedding model reference
    pub fn embedding_model(&self) -> &TextEmbeddingModel {
        &self.embedding_model
    }

    /// Update default search options
    pub fn set_default_options(&mut self, options: SearchOptions) {
        self.default_options = options;
    }

    /// Get current default options
    pub fn default_options(&self) -> &SearchOptions {
        &self.default_options
    }
}
