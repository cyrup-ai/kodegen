//! Decay worker implementation
//!
//! Implements continuous batch processing:
//! 1. Wake every N seconds
//! 2. Query batch of memories using cursor pagination
//! 3. Apply temporal decay to memory nodes (importance + quantum coherence)
//! 4. Query and decay associated entanglement/causal edges
//! 5. Persist changes to SurrealDB
//! 6. Repeat with next batch

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::Utc;
use futures_util::StreamExt;

use crate::memory::core::manager::coordinator::MemoryCoordinator;
use crate::memory::core::manager::surreal::trait_def::MemoryManager;
use crate::memory::utils::Result;

use super::config::DecayWorkerConfig;

/// Background worker for temporal decay processing
#[derive(Debug)]
pub struct DecayWorker {
    coordinator: Arc<MemoryCoordinator>,
    config: DecayWorkerConfig,
    cursor: Arc<AtomicUsize>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
}

impl DecayWorker {
    /// Create new decay worker
    pub fn new(
        coordinator: Arc<MemoryCoordinator>,
        config: DecayWorkerConfig,
        shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            coordinator,
            config,
            cursor: Arc::new(AtomicUsize::new(0)),
            shutdown_rx,
        }
    }

    /// Run the decay worker loop
    pub async fn run(mut self) {
        let cycle_interval = Duration::from_secs(self.config.cycle_interval_secs);

        loop {
            tokio::select! {
                _ = tokio::time::sleep(cycle_interval) => {
                    log::debug!("Decay worker cycle starting");

                    // Process one batch
                    match self.process_batch().await {
                        Ok(processed_count) => {
                            log::debug!("Decay worker processed {} memories", processed_count);
                        }
                        Err(e) => {
                            log::error!("Decay worker batch processing failed: {}", e);
                        }
                    }
                }
                _ = self.shutdown_rx.changed() => {
                    log::info!("Decay worker received shutdown signal");
                    break;
                }
            }
        }

        log::info!("Decay worker stopped gracefully");
    }

    /// Process a single batch of memories
    async fn process_batch(&self) -> Result<usize> {
        let offset = self.cursor.load(Ordering::Relaxed);
        let limit = self.config.batch_size;

        // Query batch of memories using pagination
        let memory_stream = self
            .coordinator
            .surreal_manager
            .list_all_memories(limit, offset);

        let memories: Vec<_> = memory_stream.collect().await;

        let memory_count = memories.len();

        if memory_count == 0 {
            // Reached end of memories, reset cursor
            log::debug!("Decay worker reached end, resetting cursor");
            self.cursor.store(0, Ordering::Relaxed);
            return Ok(0);
        }

        let mut processed_count = 0;

        for memory_result in memories {
            match memory_result {
                Ok(memory_node) => {
                    // Check minimum age requirement
                    let age = Utc::now().signed_duration_since(memory_node.created_at);
                    let hours_old = age.num_seconds() as f64 / 3600.0;

                    if hours_old < self.config.min_age_hours as f64 {
                        // Skip memories that are too fresh
                        continue;
                    }

                    // Process this memory
                    if let Err(e) = self.process_memory(&memory_node).await {
                        log::warn!("Failed to process memory {}: {}", memory_node.id, e);
                    } else {
                        processed_count += 1;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to retrieve memory from batch: {}", e);
                }
            }
        }

        // Advance cursor for next batch only if we got results
        if memory_count > 0 {
            self.cursor.fetch_add(limit, Ordering::Relaxed);
        }

        Ok(processed_count)
    }

    /// Process a single memory: apply decay to node and edges
    async fn process_memory(
        &self,
        memory_node: &crate::memory::core::primitives::node::MemoryNode,
    ) -> Result<()> {
        // Step 1: Apply temporal decay to memory node
        let mut domain_memory = self
            .coordinator
            .convert_memory_to_domain_node(memory_node)?;

        self.coordinator
            .apply_temporal_decay(&mut domain_memory)
            .await?;

        // Convert back and persist
        let updated_memory = self
            .coordinator
            .convert_domain_to_memory_node(&domain_memory);
        self.coordinator
            .surreal_manager
            .update_memory(updated_memory)
            .await?;

        // Step 2: Apply decay to entanglement edges
        self.decay_entanglement_edges(&memory_node.id).await?;

        // Step 3: Apply decay to causal edges
        self.decay_causal_edges(&memory_node.id).await?;

        Ok(())
    }

    /// Generic method to apply temporal decay to edge strengths (entangled or caused)
    async fn decay_edges_generic(
        &self,
        memory_id: &str,
        table_name: &str,
        error_context: &str,
    ) -> Result<()> {
        use std::collections::HashMap;
        use surrealdb::RecordId;

        // Query all edges for this memory
        let query = format!(
            "SELECT id, in, out, strength, created_at FROM {} WHERE in = $memory_id OR out = $memory_id",
            table_name
        );
        let memory_id_owned = memory_id.to_string();

        let db = &self.coordinator.surreal_manager.db;
        let mut response = db
            .query(&query)
            .bind(("memory_id", memory_id_owned))
            .await
            .map_err(|e| crate::memory::utils::Error::Database(format!("{:?}", e)))?;

        #[derive(serde::Deserialize)]
        struct EdgeData {
            id: RecordId,
            #[serde(rename = "in")]
            source: RecordId,
            #[serde(rename = "out")]
            target: RecordId,
            strength: f32,
            created_at: u64,
        }

        let edges: Vec<EdgeData> = response.take(0).map_err(|e| {
            log::error!(
                "Failed to parse edge data for {} edges (memory {}): {:?}",
                error_context,
                memory_id,
                e
            );
            crate::memory::utils::Error::Database(format!(
                "Edge deserialization failed for {} edges: {:?}",
                error_context, e
            ))
        })?;

        if edges.is_empty() {
            return Ok(());
        }

        // Track edge updates for cache synchronization
        let mut edge_updates: HashMap<(String, String), f64> = HashMap::new();

        // Apply exponential decay to each edge and build batch update query
        let now = Utc::now();
        let decay_rate = self.coordinator.get_decay_rate();

        let mut update_statements = Vec::new();
        let mut new_strengths = Vec::new();
        let mut valid_edges: Vec<&EdgeData> = Vec::new(); // Track valid edge references
        let mut valid_idx = 0; // Separate counter for valid edges only

        for edge in edges.iter() {
            let created_time =
                match chrono::DateTime::<Utc>::from_timestamp_millis(edge.created_at as i64) {
                    Some(time) => time,
                    None => {
                        log::warn!(
                            "Invalid timestamp {} for {} edge {}, skipping",
                            edge.created_at,
                            error_context,
                            edge.id
                        );
                        continue; // Skip this edge
                    }
                };

            let age = now.signed_duration_since(created_time);
            let days_old = age.num_seconds() as f64 / 86400.0;

            // Calculate decayed strength: strength * e^(-decay_rate * days)
            let decay_factor = (-decay_rate * days_old).exp();
            let new_strength = (edge.strength as f64 * decay_factor).max(0.01) as f32;

            // Build UPDATE statement for this edge
            update_statements.push(format!(
                "UPDATE {} SET strength = $strength_{};",
                edge.id, valid_idx
            ));
            new_strengths.push(new_strength);
            valid_edges.push(edge); // Store reference to valid edge
            valid_idx += 1; // Only increment for valid edges
        }

        // Execute batch UPDATE query - GUARANTEE success
        let batch_query = update_statements.join("\n");
        let mut query_builder = db.query(&batch_query);

        for (idx, &strength) in new_strengths.iter().enumerate() {
            query_builder = query_builder.bind((format!("strength_{}", idx), strength));
        }

        let mut response = query_builder.await.map_err(|e| {
            crate::memory::utils::Error::Database(format!(
                "Failed to batch update {} edges: {:?}",
                error_context, e
            ))
        })?;

        // Verify each UPDATE statement succeeded - CRITICAL for cache sync correctness
        for idx in 0..valid_idx {
            response.take::<Vec<serde_json::Value>>(idx).map_err(|e| {
                crate::memory::utils::Error::Database(format!(
                    "UPDATE statement {} failed for {} edge: {:?}",
                    idx, error_context, e
                ))
            })?;
        }

        // Batch UPDATE succeeded - now safe to track for cache synchronization
        for (edge, &new_strength) in valid_edges.iter().zip(new_strengths.iter()) {
            let src = edge.source.to_string();
            let tgt = edge.target.to_string();
            edge_updates.insert((src, tgt), new_strength as f64);
        }

        // Synchronize in-memory quantum state cache
        if !edge_updates.is_empty() {
            let mut state = self.coordinator.quantum_state.write().await;
            for link in state.entanglement_links.iter_mut() {
                // HashMap O(1) lookup - string clones are cheap (refcount increment)
                if let Some(&new_strength) = edge_updates
                    .get(&(link.node_a.clone(), link.node_b.clone()))
                    .or_else(|| edge_updates.get(&(link.node_b.clone(), link.node_a.clone())))
                {
                    link.entanglement_strength = new_strength;
                }
            }
        }

        Ok(())
    }

    /// Apply temporal decay to entanglement edge strengths
    async fn decay_entanglement_edges(&self, memory_id: &str) -> Result<()> {
        self.decay_edges_generic(memory_id, "entangled", "entanglement")
            .await
    }

    /// Apply temporal decay to causal edge strengths
    async fn decay_causal_edges(&self, memory_id: &str) -> Result<()> {
        self.decay_edges_generic(memory_id, "caused", "causal")
            .await
    }
}
