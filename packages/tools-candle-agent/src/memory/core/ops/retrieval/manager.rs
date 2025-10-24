//! Memory retrieval manager orchestrating multiple strategies

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::stream::StreamExt;

use crate::memory::filter::MemoryFilter;
use crate::memory::utils::Result;
use crate::memory::vector::VectorStore;

use super::semantic::SemanticRetrieval;
use super::strategy::RetrievalStrategy;
use super::temporal::TemporalRetrieval;
use super::types::RetrievalResult;

/// Memory retrieval manager
pub struct RetrievalManager<V: VectorStore> {
    strategies: HashMap<String, Arc<dyn RetrievalStrategy>>,
    default_strategy: String,
    vector_store: V,
}

impl<V: VectorStore + Clone + Send + Sync + 'static> RetrievalManager<V> {
    /// Create a new retrieval manager
    pub fn new(vector_store: V) -> Self {
        let mut strategies: HashMap<String, Arc<dyn RetrievalStrategy>> = HashMap::new();

        // Add default strategies
        strategies.insert(
            "semantic".to_string(),
            Arc::new(SemanticRetrieval::new(vector_store.clone())),
        );

        // Note: Temporal retrieval requires a memory manager to be added later
        // via add_temporal_strategy method

        Self {
            strategies,
            default_strategy: "semantic".to_string(),
            vector_store,
        }
    }

    /// Add temporal retrieval strategy with a memory manager
    pub fn add_temporal_strategy(
        &mut self,
        memory_manager: Arc<dyn crate::memory::MemoryManager>,
        time_decay_factor: f32,
    ) {
        self.strategies.insert(
            "temporal".to_string(),
            Arc::new(TemporalRetrieval::new(time_decay_factor, memory_manager)),
        );
    }

    /// Set the default retrieval strategy
    pub fn set_default_strategy(&mut self, strategy_name: String) {
        self.default_strategy = strategy_name;
    }

    /// Add a custom retrieval strategy
    pub fn add_strategy(&mut self, name: String, strategy: Arc<dyn RetrievalStrategy>) {
        self.strategies.insert(name, strategy);
    }

    /// Direct vector search using the managed vector store
    pub async fn direct_vector_search(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<crate::memory::vector::VectorSearchResult>> {
        let filter = crate::memory::filter::MemoryFilter::new();
        let search_stream = self.vector_store.search(query_vector, limit, Some(filter));

        // Collect all results from the stream
        let results: Vec<_> = search_stream.collect().await;

        Ok(results)
    }

    /// Retrieve memories using the specified strategy
    pub async fn retrieve(
        &self,
        query: &str,
        strategy_name: Option<&str>,
        limit: usize,
        filter: Option<&MemoryFilter>,
    ) -> Result<Vec<RetrievalResult>> {
        let strategy_name = strategy_name.unwrap_or(&self.default_strategy);

        if let Some(strategy) = self.strategies.get(strategy_name) {
            strategy
                .retrieve(query.to_string(), limit, filter.cloned())
                .await
        } else {
            Err(crate::memory::utils::error::Error::InvalidInput(format!(
                "Unknown retrieval strategy: {strategy_name}"
            )))
        }
    }

    /// Retrieve using multiple strategies and combine results
    pub async fn multi_strategy_retrieve(
        &self,
        query: &str,
        strategy_names: Vec<&str>,
        limit: usize,
        filter: Option<&MemoryFilter>,
    ) -> Result<Vec<RetrievalResult>> {
        let mut all_results = Vec::new();

        for strategy_name in strategy_names {
            if let Some(strategy) = self.strategies.get(strategy_name) {
                let results = strategy
                    .retrieve(query.to_string(), limit, filter.cloned())
                    .await?;
                all_results.extend(results);
            }
        }

        // Deduplicate and sort by score
        let mut unique_results: HashMap<String, RetrievalResult> = HashMap::new();
        for result in all_results {
            unique_results
                .entry(result.id.clone())
                .and_modify(|r| {
                    if result.score > r.score {
                        r.score = result.score;
                    }
                })
                .or_insert(result);
        }

        let mut sorted_results: Vec<_> = unique_results.into_values().collect();
        sorted_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted_results.truncate(limit);

        Ok(sorted_results)
    }
}
