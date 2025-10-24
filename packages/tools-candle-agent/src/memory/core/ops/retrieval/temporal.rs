//! Temporal proximity retrieval with production-ready database integration

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::stream::StreamExt;
use tokio::sync::oneshot;

use crate::memory::filter::MemoryFilter;
use crate::memory::utils::Result;

use super::strategy::RetrievalStrategy;
use super::types::{PendingRetrieval, RetrievalMethod, RetrievalResult};

/// Temporal proximity retrieval with production-ready database integration
pub struct TemporalRetrieval {
    time_decay_factor: f32,
    memory_manager: Arc<dyn crate::memory::MemoryManager>,
}

impl TemporalRetrieval {
    /// Create a new temporal retrieval strategy with memory manager
    pub fn new(
        time_decay_factor: f32,
        memory_manager: Arc<dyn crate::memory::MemoryManager>,
    ) -> Self {
        Self {
            time_decay_factor,
            memory_manager,
        }
    }
}

impl RetrievalStrategy for TemporalRetrieval {
    fn retrieve(
        &self,
        query: String,
        limit: usize,
        filter: Option<MemoryFilter>,
    ) -> PendingRetrieval {
        let (tx, rx) = oneshot::channel();
        let time_decay = self.time_decay_factor;
        let memory_manager = self.memory_manager.clone();

        tokio::spawn(async move {
            let result: Result<Vec<RetrievalResult>> = (async {
                // Create temporal filter - prioritize recent memories
                let mut temporal_filter = filter.unwrap_or_default();

                // Set time window to last 30 days if not specified
                if temporal_filter.time_range.is_none() {
                    let now = chrono::Utc::now();
                    let thirty_days_ago = now - chrono::Duration::days(30);
                    temporal_filter.time_range = Some(crate::memory::filter::TimeRange {
                        start: Some(thirty_days_ago),
                        end: Some(now),
                    });
                }

                // Apply limit multiplier to get enough results for filtering
                temporal_filter.limit = Some(limit * 3);

                // Query content-based memories first if query is provided
                let mut all_memories = Vec::new();

                if !query.is_empty() {
                    // Use content search as primary filter
                    let content_stream = memory_manager.search_by_content(&query);
                    let mut content_memories = Vec::new();

                    tokio::pin!(content_stream);
                    while let Some(memory_result) = content_stream.next().await {
                        match memory_result {
                            Ok(memory) => {
                                // Apply time range filter manually
                                if let Some(ref time_range) = temporal_filter.time_range {
                                    let created_at = memory.created_at;
                                    let in_range =
                                        time_range.start.is_none_or(|start| created_at >= start)
                                            && time_range.end.is_none_or(|end| created_at < end);

                                    if in_range {
                                        content_memories.push(memory);
                                    }
                                } else {
                                    content_memories.push(memory);
                                }
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }

                        if content_memories.len() >= temporal_filter.limit.unwrap_or(limit * 3) {
                            break;
                        }
                    }

                    all_memories.extend(content_memories);
                }

                // If we don't have enough memories, get recent ones by type
                if all_memories.len() < limit {
                    let remaining = limit * 3 - all_memories.len();
                    let memory_types = temporal_filter.memory_types.clone().unwrap_or_else(|| {
                        vec![
                            crate::memory::primitives::types::MemoryTypeEnum::Episodic,
                            crate::memory::primitives::types::MemoryTypeEnum::Semantic,
                            crate::memory::primitives::types::MemoryTypeEnum::Procedural,
                        ]
                    });

                    for memory_type in memory_types {
                        let type_stream = memory_manager.query_by_type(memory_type);
                        let mut type_memories = Vec::new();

                        tokio::pin!(type_stream);
                        while let Some(memory_result) = type_stream.next().await {
                            match memory_result {
                                Ok(memory) => {
                                    // Apply time range filter manually
                                    if let Some(ref time_range) = temporal_filter.time_range {
                                        let created_at = memory.created_at;
                                        let in_range = time_range
                                            .start
                                            .is_none_or(|start| created_at >= start)
                                            && time_range.end.is_none_or(|end| created_at < end);

                                        if in_range {
                                            type_memories.push(memory);
                                        }
                                    } else {
                                        type_memories.push(memory);
                                    }
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }

                            if type_memories.len() >= remaining {
                                break;
                            }
                        }

                        all_memories.extend(type_memories);

                        if all_memories.len() >= limit * 3 {
                            break;
                        }
                    }
                }

                // Apply temporal scoring with decay factor
                let now = chrono::Utc::now();
                let now_timestamp = now.timestamp() as f64;

                let mut scored_results: Vec<RetrievalResult> = all_memories
                    .into_iter()
                    .map(|memory| {
                        // Calculate age in hours
                        let memory_timestamp = memory.created_at.timestamp() as f64;
                        let age_hours = (now_timestamp - memory_timestamp) / 3600.0;

                        // Apply exponential decay based on age
                        let temporal_score = if age_hours >= 0.0 {
                            -(age_hours * time_decay as f64).exp() as f32
                        } else {
                            1.0 // Future memories get max score
                        };

                        // Base relevance score (could be enhanced with content matching)
                        let relevance_score = if !query.is_empty() {
                            // Simple content matching score
                            let content_lower = memory.content.text.to_lowercase();
                            let query_lower = query.to_lowercase();

                            if content_lower.contains(&query_lower) {
                                let match_ratio =
                                    query_lower.len() as f32 / content_lower.len() as f32;
                                (match_ratio * 2.0).min(1.0)
                            } else {
                                0.1 // Minimal base score for type-based matches
                            }
                        } else {
                            0.5 // Default relevance for temporal-only queries
                        };

                        // Combine temporal and relevance scores
                        let final_score = (temporal_score * 0.7) + (relevance_score * 0.3);

                        // Create metadata with temporal information
                        let mut metadata = HashMap::new();
                        metadata.insert(
                            "age_hours".to_string(),
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(age_hours)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            ),
                        );
                        metadata.insert(
                            "temporal_score".to_string(),
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(temporal_score as f64)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            ),
                        );
                        metadata.insert(
                            "relevance_score".to_string(),
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(relevance_score as f64)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            ),
                        );
                        metadata.insert(
                            "created_at".to_string(),
                            serde_json::Value::String(memory.created_at.to_rfc3339()),
                        );

                        RetrievalResult {
                            id: memory.id,
                            score: final_score,
                            method: RetrievalMethod::Temporal,
                            metadata,
                        }
                    })
                    .collect();

                // Sort by score (descending) and take top results
                scored_results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                scored_results.truncate(limit);

                Ok(scored_results)
            })
            .await;

            let _ = tx.send(result);
        });

        PendingRetrieval::new(rx)
    }

    fn name(&self) -> &str {
        "temporal"
    }
}
