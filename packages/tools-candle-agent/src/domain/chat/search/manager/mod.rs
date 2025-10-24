use std::sync::Arc;

use crossbeam_skiplist::SkipMap;
use std::pin::Pin;
use tokio_stream::Stream;

use super::{SearchQuery as CandleSearchQuery, SearchResult as CandleSearchResult};
use crate::domain::chat::message::CandleSearchChatMessage;

/// Enhanced history management system with domain-level integration
#[derive(Debug)]
pub struct CandleEnhancedHistoryManager {
    /// Search index
    search_index: Arc<super::index::ChatSearchIndex>,
    /// Conversation tagger
    tagger: Arc<super::tagger::CandleConversationTagger>,
    /// History exporter
    #[allow(dead_code)] // TODO: Implement history export functionality
    exporter: Arc<super::export::HistoryExporter>,
    /// Message store
    messages: Arc<SkipMap<String, CandleSearchChatMessage>>,
    /// Message index by timestamp
    message_timestamps: Arc<SkipMap<i64, String>>,
}

impl Default for CandleEnhancedHistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CandleEnhancedHistoryManager {
    /// Create a new enhanced history manager
    #[must_use]
    pub fn new() -> Self {
        let tagger = Arc::new(super::tagger::CandleConversationTagger::new());
        let mut search_index = super::index::ChatSearchIndex::new();
        search_index.tagger = Some(Arc::clone(&tagger));

        Self {
            search_index: Arc::new(search_index),
            tagger,
            exporter: Arc::new(super::export::HistoryExporter::new()),
            messages: Arc::new(SkipMap::new()),
            message_timestamps: Arc::new(SkipMap::new()),
        }
    }

    /// Add message to history manager (streaming)
    #[must_use]
    pub fn add_message_stream(
        &self,
        message: &CandleSearchChatMessage,
    ) -> Pin<Box<dyn Stream<Item = super::types::IndexOperationResult> + Send>> {
        let message_id = message.message.id.clone();
        let timestamp = message.message.timestamp;
        let message_clone = message.clone();
        let messages = Arc::clone(&self.messages);
        let message_timestamps = Arc::clone(&self.message_timestamps);
        let search_index: Arc<super::index::ChatSearchIndex> = Arc::clone(&self.search_index);

        // Clone tagger for auto-tagging
        let tagger = Arc::clone(&self.tagger);
        let message_content = message.message.content.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Add to message store - only if message_id is present
            if let Some(id) = message_id.clone() {
                messages.insert(id.clone(), message_clone.clone());

                // Index by timestamp - only if timestamp is present
                if let Some(ts_u64) = timestamp {
                    #[allow(clippy::cast_possible_wrap)]
                    let ts_i64 = ts_u64 as i64;
                    message_timestamps.insert(ts_i64, id.clone());
                }

                // Auto-tag the message
                tagger.auto_tag_message(&id, &message_content);
            }

            // Index for search
            let _ = search_index.add_message_stream(message_clone);

            // Emit completion
            let result = super::types::IndexOperationResult {
                success: true,
                document_id: message_id.unwrap_or_else(|| "unknown".to_string()),
                terms_indexed: 0, // Manager doesn't index terms, just stores messages
                duration_ms: 0.0, // Could measure duration if needed
                error: None,
            };
            let _ = tx.send(result);
        }))
    }

    /// Search messages (streaming)
    #[must_use]
    pub fn search_messages_stream(
        &self,
        query: &CandleSearchQuery,
    ) -> Pin<Box<dyn Stream<Item = CandleSearchResult> + Send>> {
        let search_index = Arc::clone(&self.search_index);
        let query_clone = query.clone();

        // Create ChatSearcher with the index
        let searcher = super::ChatSearcher::new(search_index);

        // Delegate to ChatSearcher which has full implementation
        searcher.search_stream(query_clone)
    }

    /// Search messages by tag
    #[must_use]
    pub fn search_by_tag(&self, tag: &str) -> Vec<CandleSearchChatMessage> {
        // Find tag ID by name
        let tag_id: Option<String> = self
            .tagger
            .tags
            .iter()
            .find(|e| e.value().name == tag)
            .map(|e| e.key().clone());

        if let Some(tid) = tag_id {
            let message_ids = self.tagger.get_messages_by_tag(&tid);
            message_ids
                .iter()
                .filter_map(|msg_id| self.messages.get(msg_id).map(|e| e.value().clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get tags for a specific message
    #[must_use]
    pub fn get_message_tags(&self, message_id: &str) -> Vec<String> {
        let tag_ids = self.tagger.get_tags(message_id);
        tag_ids
            .iter()
            .filter_map(|tag_id| self.tagger.tags.get(tag_id).map(|e| e.value().name.clone()))
            .collect()
    }

    /// Get all tags with usage counts
    #[must_use]
    pub fn get_all_tags(&self) -> Vec<(String, usize)> {
        self.tagger.get_all_tags()
    }

    /// Get tag statistics
    #[must_use]
    pub fn tag_statistics(&self) -> super::tagger::CandleTaggingStatistics {
        self.tagger.tag_statistics()
    }
}
