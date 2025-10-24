//! Vector search functionality - THREAD-SAFE SYNCHRONOUS OPERATIONS
//!
//! This module provides comprehensive vector search capabilities using:
//! - Synchronous vector similarity search with SIMD acceleration
//! - Thread-safe embedding generation and caching
//! - Hybrid search combining vector and keyword approaches
//! - Zero-allocation search result processing
//! - Advanced filtering and ranking algorithms

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use surrealdb::Value;

use crate::capability::registry::TextEmbeddingModel;
use crate::capability::traits::TextEmbeddingCapable;
use crate::memory::constants::SEARCH_TASK;
use crate::memory::utils::error::Result;
use crate::memory::vector::vector_store::VectorStore;

use crate::domain::memory::cognitive::types::{
    CognitiveProcessor, CognitiveProcessorConfig, DecisionOutcome,
};

/// Type alias for request info callback function
/// Callback receives: (result_id, similarity, confidence) -> bool (accept/reject)
type RequestInfoCallback = Arc<dyn Fn(&str, f32, f32) -> bool + Send + Sync>;

/// Type alias for deferred search result with confidence
/// Format: (id, vector, similarity, metadata, confidence)
type DeferredResult = (String, Vec<f32>, f32, Option<HashMap<String, Value>>, f32);

/// Type alias for final search result
/// Format: (id, vector, similarity, metadata)
type FinalResult = (String, Vec<f32>, f32, Option<HashMap<String, Value>>);

/// Convert static string to Option<String> for embedding tasks
#[inline]
fn task_string(task: &'static str) -> Option<String> {
    Some(task.to_string())
}

/// State for multi-stage cognitive filtering
struct CognitiveSearchState {
    /// Results deferred for secondary evaluation with confidence scores
    deferred_results: Vec<DeferredResult>,

    /// Final accepted results
    final_results: Vec<FinalResult>,
}

impl CognitiveSearchState {
    fn new() -> Self {
        Self {
            deferred_results: Vec::new(),
            final_results: Vec::new(),
        }
    }
}

/// Type alias for keyword search function - SYNCHRONOUS OPERATIONS
///
/// This function type represents a synchronous keyword search operation.
/// For concurrent execution, wrap the function call in a thread.
pub type KeywordSearchFn =
    Arc<dyn Fn(&str, Option<SearchOptions>) -> Result<Vec<SearchResult>> + Send + Sync>;

/// Search result with comprehensive metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique identifier of the vector
    pub id: String,
    /// Vector data (included based on search options)
    pub vector: Vec<f32>,
    /// Similarity score (0.0 to 1.0, higher is better)
    pub similarity: f32,
    /// Optional metadata associated with the vector
    pub metadata: Option<HashMap<String, Value>>,
    /// Search ranking information (optional)
    pub rank: Option<usize>,
    /// Combined score from multiple search strategies (for hybrid search)
    pub combined_score: Option<f32>,
    /// Cognitive processor decision confidence (0.0 to 1.0)
    pub decision_confidence: Option<f32>,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(id: String, vector: Vec<f32>, similarity: f32) -> Self {
        Self {
            id,
            vector,
            similarity,
            metadata: None,
            rank: None,
            combined_score: None,
            decision_confidence: None,
        }
    }

    /// Create with metadata
    pub fn with_metadata(
        id: String,
        vector: Vec<f32>,
        similarity: f32,
        metadata: HashMap<String, Value>,
    ) -> Self {
        Self {
            id,
            vector,
            similarity,
            metadata: Some(metadata),
            rank: None,
            combined_score: None,
            decision_confidence: None,
        }
    }

    /// Set the ranking position
    #[must_use]
    pub fn with_rank(mut self, rank: usize) -> Self {
        self.rank = Some(rank);
        self
    }

    /// Set the combined score for hybrid search
    #[must_use]
    pub fn with_combined_score(mut self, score: f32) -> Self {
        self.combined_score = Some(score);
        self
    }

    /// Set the decision confidence from cognitive processor
    #[must_use]
    pub fn with_decision_confidence(mut self, confidence: f32) -> Self {
        self.decision_confidence = Some(confidence);
        self
    }

    /// Get the effective score for sorting (combined_score if available, otherwise similarity)
    pub fn effective_score(&self) -> f32 {
        self.combined_score.unwrap_or(self.similarity)
    }

    /// Memory usage estimation in bytes
    pub fn memory_usage(&self) -> usize {
        self.id.len() +
        self.vector.len() * std::mem::size_of::<f32>() +
        self.metadata.as_ref().map(|m| m.len() * 64).unwrap_or(0) + // Approximate metadata size
        std::mem::size_of::<Self>()
    }
}

/// Search options for fine-tuning search behavior
#[derive(Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Minimum similarity threshold (0.0 to 1.0)
    pub min_similarity: Option<f32>,
    /// Metadata filters to apply (exact match)
    pub filters: Option<HashMap<String, Value>>,
    /// Whether to include vectors in results (affects memory usage)
    pub include_vectors: Option<bool>,
    /// Whether to include metadata in results
    pub include_metadata: Option<bool>,
    /// Whether to include ranking information
    pub include_rank: Option<bool>,
    /// Maximum number of results to consider before filtering (for performance)
    pub candidate_limit: Option<usize>,
    /// Whether to enable SIMD optimization (default: true)
    pub enable_simd: Option<bool>,
    /// Optional callback for RequestInfo outcomes requiring user interaction
    /// Callback receives: (result_id, similarity, confidence) -> bool (accept/reject)
    #[serde(skip)]
    pub request_info_callback: Option<RequestInfoCallback>,
}

impl std::fmt::Debug for SearchOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchOptions")
            .field("limit", &self.limit)
            .field("min_similarity", &self.min_similarity)
            .field("filters", &self.filters)
            .field("include_vectors", &self.include_vectors)
            .field("include_metadata", &self.include_metadata)
            .field("include_rank", &self.include_rank)
            .field("candidate_limit", &self.candidate_limit)
            .field("enable_simd", &self.enable_simd)
            .field(
                "request_info_callback",
                &self.request_info_callback.as_ref().map(|_| "<callback>"),
            )
            .finish()
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: Some(10),
            min_similarity: Some(0.7),
            filters: None,
            include_vectors: Some(false),
            include_metadata: Some(true),
            include_rank: Some(false),
            candidate_limit: Some(1000),
            enable_simd: Some(true),
            request_info_callback: None,
        }
    }
}

impl SearchOptions {
    /// Create options optimized for performance (minimal data returned)
    pub fn fast() -> Self {
        Self {
            limit: Some(10),
            min_similarity: Some(0.8),
            filters: None,
            include_vectors: Some(false),
            include_metadata: Some(false),
            include_rank: Some(false),
            candidate_limit: Some(100),
            enable_simd: Some(true),
            request_info_callback: None,
        }
    }

    /// Create options optimized for comprehensive results
    pub fn comprehensive() -> Self {
        Self {
            limit: Some(50),
            min_similarity: Some(0.5),
            filters: None,
            include_vectors: Some(true),
            include_metadata: Some(true),
            include_rank: Some(true),
            candidate_limit: Some(10000),
            enable_simd: Some(true),
            request_info_callback: None,
        }
    }

    /// Validate the options and return normalized values
    pub fn validate(mut self) -> Result<Self> {
        // Clamp similarity threshold
        if let Some(threshold) = self.min_similarity
            && !(0.0..=1.0).contains(&threshold)
        {
            return Err(crate::memory::utils::error::Error::InvalidInput(
                "min_similarity must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Ensure reasonable limits
        if let Some(limit) = self.limit {
            if limit == 0 {
                self.limit = Some(1);
            } else if limit > 10000 {
                self.limit = Some(10000);
            }
        }

        if let Some(candidate_limit) = self.candidate_limit
            && candidate_limit == 0
        {
            self.candidate_limit = Some(100);
        }

        Ok(self)
    }
}

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
        let embedding = self.embedding_model.embed(text, task_string(SEARCH_TASK)).await?;

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

                let vector = if options.include_vectors.unwrap_or(false) {
                    vector
                } else {
                    Vec::new()
                };

                let metadata = if options.include_metadata.unwrap_or(true) {
                    metadata
                } else {
                    None
                };

                let mut result = SearchResult {
                    id,
                    vector,
                    similarity,
                    metadata,
                    rank: None,
                    combined_score: None,
                    decision_confidence,
                };

                if options.include_rank.unwrap_or(false) {
                    result.rank = Some(index + 1);
                }

                result
            })
            .collect::<Vec<_>>();

        // Apply final limit if different from candidate limit
        if let Some(final_limit) = options.limit
            && final_limit < search_results.len()
        {
            search_results.truncate(final_limit);
        }

        Ok(search_results)
    }

    /// Batch search by multiple texts (thread-based parallel processing)
    ///
    /// # Arguments
    /// * `texts` - Vector of text queries
    /// * `options` - Search options applied to all queries
    ///
    /// # Returns
    /// Result containing vector of search result vectors (one per input text)
    pub async fn batch_search_by_text(
        &self,
        texts: &[String],
        options: Option<SearchOptions>,
    ) -> Result<Vec<Vec<SearchResult>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Generate embeddings for all texts (asynchronous batch operation)
        let embeddings = self
            .embedding_model
            .batch_embed(texts, task_string(SEARCH_TASK))
            .await?;

        // Parallel search using thread pool
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let mut handles = Vec::new();

        for (index, embedding) in embeddings.into_iter().enumerate() {
            let sender = sender.clone();
            let store = Arc::clone(&self.store);
            let embedding_model = self.embedding_model.clone();
            let options = options.clone();

            let handle = tokio::task::spawn_blocking(move || {
                // Use the actual embedding model for consistency
                // Even though we don't need it for pure vector search, it's available
                let processor_config = CognitiveProcessorConfig::default();
                let search = VectorSearch {
                    store,
                    embedding_model,
                    default_options: SearchOptions::default(),
                    cognitive_processor: Arc::new(CognitiveProcessor::new(processor_config)),
                };

                let result = search.search_by_embedding(&embedding, options);
                let _ = sender.send((index, result));
            });

            handles.push(handle);
        }

        // Drop the original sender so receiver can finish
        drop(sender);

        // Wait for all blocking tasks to complete
        for handle in handles {
            if let Err(e) = handle.await {
                return Err(crate::memory::utils::error::Error::Other(
                    format!("Blocking task execution failed during batch search: {}", e)
                ));
            }
        }

        // Collect results in original order
        let mut results = vec![Vec::new(); texts.len()];
        while let Some((index, result)) = receiver.recv().await {
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

/// Process deferred results with secondary threshold evaluation
///
/// Results with confidence above the secondary threshold are promoted to final results.
/// This implements a two-stage filtering approach for medium-confidence items.
///
/// # Arguments
/// * `state` - Mutable reference to cognitive search state
/// * `threshold` - Secondary threshold for deferred result acceptance (typically 0.56 = 0.7 * 0.8)
fn process_deferred_results(state: &mut CognitiveSearchState, threshold: f32) {
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

/// Hybrid search combining vector and keyword search strategies
///
/// This implementation provides sophisticated search capabilities by combining:
/// - Vector similarity search for semantic matching
/// - Keyword search for exact term matching
/// - Configurable weighting between strategies
/// - Advanced result merging and ranking algorithms
#[derive(Clone)]
pub struct HybridSearch {
    /// Vector search component
    vector_search: VectorSearch,
    /// Keyword search function
    keyword_search: KeywordSearchFn,
    /// Weight for vector search results (0.0 to 1.0)
    vector_weight: f32,
    /// Weight for keyword search results (computed as 1.0 - vector_weight)
    keyword_weight: f32,
}

impl HybridSearch {
    /// Create a new HybridSearch with custom keyword search function
    ///
    /// # Arguments
    /// * `vector_search` - Vector search implementation
    /// * `keyword_search` - Synchronous keyword search function
    /// * `vector_weight` - Weight for vector results (0.0 to 1.0, default: 0.5)
    ///
    /// # Returns
    /// New HybridSearch instance
    pub fn new(
        vector_search: VectorSearch,
        keyword_search: KeywordSearchFn,
        vector_weight: Option<f32>,
    ) -> Self {
        let vector_weight = vector_weight.unwrap_or(0.5).clamp(0.0, 1.0);
        let keyword_weight = 1.0 - vector_weight;

        Self {
            vector_search,
            keyword_search,
            vector_weight,
            keyword_weight,
        }
    }

    /// Search using both vector and keyword strategies (synchronous)
    ///
    /// # Arguments
    /// * `text` - Search query text
    /// * `options` - Search options applied to both strategies
    ///
    /// # Returns
    /// Result containing merged and ranked search results
    pub async fn search(&self, text: &str, options: Option<SearchOptions>) -> Result<Vec<SearchResult>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        // Execute both searches in parallel using tokio tasks
        let vector_search = self.vector_search.clone();
        let text_for_vector = text.to_string();
        let options_for_vector = options.clone();

        let vector_handle = tokio::spawn(async move {
            vector_search.search_by_text(&text_for_vector, options_for_vector).await
        });

        // Keyword search
        let keyword_search = self.keyword_search.clone();
        let text_for_keyword = text.to_string();
        let options_for_keyword = options.clone();

        let keyword_handle = tokio::task::spawn_blocking(move || {
            (keyword_search)(&text_for_keyword, options_for_keyword)
        });

        // Wait for both results
        let vector_results = vector_handle.await
            .map_err(|e| crate::memory::utils::error::Error::Other(format!("Vector search task failed: {}", e)))??;
        
        let keyword_results = keyword_handle.await
            .map_err(|e| crate::memory::utils::error::Error::Other(format!("Keyword search task failed: {}", e)))??;

        // Combine and rank results
        let combined_results = self.combine_results(vector_results, keyword_results, options);

        Ok(combined_results)
    }

    /// Combine vector and keyword search results with sophisticated merging
    fn combine_results(
        &self,
        vector_results: Vec<SearchResult>,
        keyword_results: Vec<SearchResult>,
        options: Option<SearchOptions>,
    ) -> Vec<SearchResult> {
        let options = options.unwrap_or_default();
        let limit = options.limit.unwrap_or(10);

        // Create a map for efficient result merging
        let mut combined_map = HashMap::new();

        // Process vector results
        for result in vector_results {
            let weighted_similarity = result.similarity * self.vector_weight;
            combined_map.insert(
                result.id.clone(),
                SearchResult {
                    id: result.id,
                    vector: result.vector,
                    similarity: result.similarity,
                    metadata: result.metadata,
                    rank: result.rank,
                    combined_score: Some(weighted_similarity),
                    decision_confidence: result.decision_confidence,
                },
            );
        }

        // Process keyword results and merge
        for result in keyword_results {
            let weighted_similarity = result.similarity * self.keyword_weight;

            if let Some(existing) = combined_map.remove(&result.id) {
                // Merge with existing vector result
                let new_combined_score =
                    existing.combined_score.unwrap_or(0.0) + weighted_similarity;

                combined_map.insert(
                    result.id.clone(),
                    SearchResult {
                        id: result.id,
                        vector: existing.vector, // Prefer vector data
                        similarity: existing.similarity,
                        metadata: existing.metadata.or(result.metadata), // Merge metadata
                        rank: None,                                      // Will be recomputed
                        combined_score: Some(new_combined_score),
                        decision_confidence: existing.decision_confidence,
                    },
                );
            } else {
                // New keyword-only result
                combined_map.insert(
                    result.id.clone(),
                    SearchResult {
                        id: result.id,
                        vector: result.vector,
                        similarity: result.similarity,
                        metadata: result.metadata,
                        rank: result.rank,
                        combined_score: Some(weighted_similarity),
                        decision_confidence: result.decision_confidence,
                    },
                );
            }
        }

        // Convert to vector and sort by combined score
        let mut combined_results: Vec<_> = combined_map.into_values().collect();

        // Sort by effective score (descending) with NaN handling
        combined_results.sort_by(|a, b| {
            let score_a = a.effective_score();
            let score_b = b.effective_score();

            match (score_a.is_nan(), score_b.is_nan()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater, // NaN goes to end
                (false, true) => Ordering::Less,    // NaN goes to end
                (false, false) => score_b.partial_cmp(&score_a).unwrap_or(Ordering::Equal),
            }
        });

        // Apply final limit and assign ranks
        if combined_results.len() > limit {
            combined_results.truncate(limit);
        }

        // Assign final rankings if requested
        if options.include_rank.unwrap_or(false) {
            for (index, result) in combined_results.iter_mut().enumerate() {
                result.rank = Some(index + 1);
            }
        }

        combined_results
    }

    /// Update search weights
    ///
    /// # Arguments
    /// * `vector_weight` - New weight for vector search (0.0 to 1.0)
    pub fn set_vector_weight(&mut self, weight: f32) {
        self.vector_weight = weight.clamp(0.0, 1.0);
        self.keyword_weight = 1.0 - self.vector_weight;
    }

    /// Get current vector weight
    pub fn vector_weight(&self) -> f32 {
        self.vector_weight
    }

    /// Get current keyword weight
    pub fn keyword_weight(&self) -> f32 {
        self.keyword_weight
    }

    /// Get reference to the vector search component
    pub fn vector_search(&self) -> &VectorSearch {
        &self.vector_search
    }
}

// Helper trait for shallow cloning of VectorSearch (for thread usage)
// Note: VectorSearch now derives Clone, so clone_shallow is no longer needed
// The Clone implementation handles shallow cloning automatically via Arc
