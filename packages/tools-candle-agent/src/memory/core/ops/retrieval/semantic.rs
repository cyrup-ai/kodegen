//! Semantic similarity retrieval using vector embeddings

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::stream::StreamExt;
use tokio::sync::{RwLock, oneshot};

use crate::domain::memory::cognitive::types::CognitiveState;
use crate::memory::filter::MemoryFilter;
use crate::memory::utils::Result;
use crate::memory::vector::VectorStore;

use super::strategy::RetrievalStrategy;
use super::types::{PendingRetrieval, RetrievalMethod, RetrievalResult};

/// Semantic similarity retrieval using vector embeddings
pub struct SemanticRetrieval<V: VectorStore> {
    vector_store: Arc<V>,
    cognitive_state: Option<Arc<RwLock<CognitiveState>>>,
}

impl<V: VectorStore> SemanticRetrieval<V> {
    pub fn new(vector_store: V) -> Self {
        Self {
            vector_store: Arc::new(vector_store),
            cognitive_state: None,
        }
    }

    pub fn with_cognitive_state(
        vector_store: V,
        cognitive_state: Arc<RwLock<CognitiveState>>,
    ) -> Self {
        Self {
            vector_store: Arc::new(vector_store),
            cognitive_state: Some(cognitive_state),
        }
    }
}

impl<V: VectorStore + Send + Sync + 'static> RetrievalStrategy for SemanticRetrieval<V> {
    fn retrieve(
        &self,
        query: String,
        limit: usize,
        filter: Option<MemoryFilter>,
    ) -> PendingRetrieval {
        let (tx, rx) = oneshot::channel();
        let vector_store = self.vector_store.clone();
        let cognitive_state = self.cognitive_state.clone();

        tokio::spawn(async move {
            let result: Result<Vec<RetrievalResult>> = (async {
                // Generate query embedding
                let query_embedding = vector_store.embed(query).await?;

                // Update cognitive state with query embedding as stimulus
                if let Some(ref cognitive_state) = cognitive_state {
                    let stimulus = query_embedding.clone();
                    match cognitive_state
                        .write()
                        .await
                        .update_activation_from_stimulus(stimulus)
                    {
                        Ok(()) => {
                            log::trace!("Updated cognitive activation from query embedding");
                        }
                        Err(e) => {
                            log::warn!("Failed to update cognitive activation from query: {}", e);
                        }
                    }
                }

                // Search in vector store
                let search_stream = vector_store.search(query_embedding, limit, filter);

                // Collect all results from the stream
                let results: Vec<_> = search_stream.collect().await;

                let retrieval_results = results
                    .into_iter()
                    .map(|r| RetrievalResult {
                        id: r.id,
                        score: r.score,
                        method: RetrievalMethod::Semantic,
                        metadata: HashMap::new(), // VectorSearchResult doesn't include metadata
                    })
                    .collect();

                Ok(retrieval_results)
            })
            .await;

            let _ = tx.send(result);
        });

        PendingRetrieval::new(rx)
    }

    fn name(&self) -> &str {
        "semantic"
    }
}
