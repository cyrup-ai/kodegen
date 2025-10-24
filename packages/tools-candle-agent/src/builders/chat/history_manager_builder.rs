use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::RwLock;
use crate::domain::chat::{
    CandleEnhancedHistoryManager as EnhancedHistoryManager, 
    CandleChatSearchIndex as ChatSearchIndex, 
    CandleConversationTagger as ConversationTagger, 
    CandleHistoryExporter as HistoryExporter, 
    CandleHistoryManagerStatistics as HistoryManagerStatistics, 
    CandleSearchStatistics as SearchStatistics
};

/// Builder for creating history managers
pub struct HistoryManagerBuilder {
    simd_threshold: usize,
    auto_tagging_enabled: bool,
    compression_enabled: bool,
}

impl HistoryManagerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            simd_threshold: 8,
            auto_tagging_enabled: true,
            compression_enabled: true,
        }
    }

    /// Set SIMD threshold
    pub fn simd_threshold(mut self, threshold: usize) -> Self {
        self.simd_threshold = threshold;
        self
    }

    /// Enable auto-tagging
    pub fn auto_tagging(mut self, enabled: bool) -> Self {
        self.auto_tagging_enabled = enabled;
        self
    }

    /// Enable compression
    pub fn compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    /// Build the history manager
    pub fn build(self) -> EnhancedHistoryManager {
        let search_index = Arc::new(ChatSearchIndex::new());
        search_index
            .simd_threshold
            .store(self.simd_threshold, Ordering::Relaxed);

        EnhancedHistoryManager {
            search_index,
            tagger: Arc::new(ConversationTagger::new()),
            exporter: Arc::new(HistoryExporter::new()),
            statistics: Arc::new(RwLock::new(HistoryManagerStatistics {
                search_stats: SearchStatistics {
                    total_messages: 0,
                    total_terms: 0,
                    total_queries: 0,
                    average_query_time: 0.0,
                    index_size: 0,
                    last_index_update: 0,
                },
                tagger_stats: Default::default(),
                exporter_stats: Default::default(),
            })),
        }
    }
}

impl Default for HistoryManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}