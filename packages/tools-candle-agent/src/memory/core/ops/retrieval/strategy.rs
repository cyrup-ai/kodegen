//! Retrieval strategy trait definition

use crate::memory::filter::MemoryFilter;

use super::types::PendingRetrieval;

/// Memory retrieval strategy trait
pub trait RetrievalStrategy: Send + Sync {
    /// Retrieve memories based on the strategy
    fn retrieve(
        &self,
        query: String,
        limit: usize,
        filter: Option<MemoryFilter>,
    ) -> PendingRetrieval;

    /// Get strategy name
    fn name(&self) -> &str;
}
