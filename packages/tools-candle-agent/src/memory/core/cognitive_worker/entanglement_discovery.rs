//! Quantum entanglement discovery
//!
//! Discovers and persists entanglement relationships between memories via similarity
//! analysis using quantum state measurements and SIMD-aligned activation patterns.

use crate::domain::memory::cognitive::types::{
    AlignedActivationPattern, CausalLink, CognitiveState, EntanglementType,
};
use crate::memory::core::manager::surreal::{MemoryManager, SurrealDBMemoryManager};
use crate::memory::monitoring::operations::{OperationTracker, OperationType};

/// Process entanglement discovery for a memory
pub(crate) async fn process_entanglement_discovery(
    memory_manager: &SurrealDBMemoryManager,
    operation_tracker: &OperationTracker,
    memory_id: &str,
) {
    let memory_id = memory_id.to_string();
    let manager = memory_manager;
    let tracker = operation_tracker;

    // Start operation tracking
    let op_id = tracker.start_operation(OperationType::EntanglementDiscovery, None);

    use futures_util::StreamExt;

    // Get source memory
    let memory = match manager.get_memory(&memory_id).await {
        Ok(Some(mem)) => mem,
        Ok(None) => {
            log::warn!("Memory {} not found for entanglement discovery", memory_id);
            tracker.fail_operation(op_id, "Memory not found".to_string());
            return;
        }
        Err(e) => {
            log::error!("Failed to retrieve memory {}: {:?}", memory_id, e);
            tracker.fail_operation(op_id, format!("Failed to retrieve: {:?}", e));
            return;
        }
    };

    // Load existing quantum signature if available (cached bonds)
    // Note: Primary edges are in entangled RELATION table, this is denormalized cache
    let existing_signature = match manager.get_quantum_signature(&memory_id).await {
        Ok(Some(state)) => {
            let bond_count = state.quantum_entanglement_bond_count().await;
            log::debug!(
                "Loaded existing quantum signature for {} with {} cached bonds",
                memory_id,
                bond_count
            );
            Some(state)
        }
        Ok(None) => {
            log::debug!("No existing quantum signature cache for {}", memory_id);
            None
        }
        Err(e) => {
            log::warn!(
                "Failed to load quantum signature for {}: {:?}",
                memory_id,
                e
            );
            None
        }
    };

    // Get embedding for similarity search
    let embedding_vec = match &memory.embedding {
        Some(emb) => emb.clone(),
        None => {
            log::warn!(
                "Memory {} has no embedding, cannot discover entanglement",
                memory_id
            );
            tracker.fail_operation(op_id, "No embedding".to_string());
            return;
        }
    };

    // Convert to SIMD-aligned activation pattern for vectorized operations
    let activation_pattern = AlignedActivationPattern::new(embedding_vec.clone());

    log::debug!(
        "Created SIMD-aligned activation pattern for {}: dimension={}, timestamp={:?}",
        memory_id,
        activation_pattern.dimension,
        activation_pattern.last_update
    );

    // Use existing signature cache if available, otherwise create new
    let state_a = if let Some(existing) = existing_signature {
        let bond_count = existing.quantum_entanglement_bond_count().await;
        log::debug!(
            "Using existing quantum state for {} (preserving {} cached bonds)",
            memory_id,
            bond_count
        );
        existing
    } else {
        match CognitiveState::with_quantum_coherence(
            &embedding_vec,
            vec![0.0; embedding_vec.len()], // Zero phases = amplitude-only state
        ) {
            Ok(state) => {
                log::debug!(
                    "Created new quantum coherent state for {} with {} dimensions",
                    memory_id,
                    embedding_vec.len()
                );
                state
            }
            Err(e) => {
                log::warn!(
                    "Failed to create quantum coherent state for {}: {}",
                    memory_id,
                    e
                );
                tracker.fail_operation(op_id, format!("Quantum state creation failed: {}", e));
                return;
            }
        }
    };

    // Search for similar memories
    let memory_stream = manager.search_by_vector(embedding_vec.clone(), 10);
    let memories: Vec<_> = memory_stream.collect().await;

    let mut entangled_count = 0;

    // Process each similar memory
    for memory_result in memories {
        let related_memory = match memory_result {
            Ok(mem) => mem,
            Err(e) => {
                log::error!("Error in similarity search: {:?}", e);
                continue;
            }
        };

        // Skip self
        if related_memory.id == memory_id {
            continue;
        }

        // Extract related memory embedding
        let related_embedding_vec = match &related_memory.embedding {
            Some(emb) => emb.clone(),
            None => {
                log::debug!(
                    "Related memory {} has no embedding, skipping",
                    related_memory.id
                );
                continue;
            }
        };

        // Convert related embedding to SIMD-aligned pattern
        let related_activation = AlignedActivationPattern::new(related_embedding_vec.clone());

        log::debug!(
            "Created SIMD-aligned pattern for related memory {}: dimension={}",
            related_memory.id,
            related_activation.dimension
        );

        // Create quantum state for related memory
        let state_b = match CognitiveState::with_quantum_coherence(
            &related_embedding_vec,
            vec![0.0; related_embedding_vec.len()],
        ) {
            Ok(state) => state,
            Err(e) => {
                log::warn!(
                    "Failed to create quantum state for related memory {}: {}",
                    related_memory.id,
                    e
                );
                continue;
            }
        };

        // Measure quantum entanglement between states
        let entanglement_strength = match state_a.measure_quantum_entanglement(&state_b) {
            Some(strength) => strength,
            None => {
                log::warn!(
                    "Dimension mismatch measuring entanglement between {} and {}",
                    memory.id,
                    related_memory.id
                );
                continue;
            }
        };

        log::debug!(
            "Quantum entanglement strength between {} and {}: {:.4}",
            memory.id,
            related_memory.id,
            entanglement_strength
        );

        // Calculate temporal distance between memories (milliseconds)
        let temporal_distance_ms: Option<i64> = {
            let created_at = memory.created_at;
            let related_created_at = related_memory.created_at;
            let duration = related_created_at.signed_duration_since(created_at);

            match duration.num_milliseconds().checked_abs() {
                Some(abs_ms) => {
                    if duration.num_milliseconds() < 0 {
                        Some(-abs_ms)
                    } else {
                        Some(abs_ms)
                    }
                }
                None => {
                    log::warn!(
                        "Temporal distance overflow between {} and {}: duration exceeds i64::MAX milliseconds",
                        memory.id,
                        related_memory.id
                    );
                    None
                }
            }
        };

        log::trace!(
            "Temporal distance {} → {}: {:?}ms {}",
            memory.id,
            related_memory.id,
            temporal_distance_ms,
            match temporal_distance_ms {
                Some(ms) if ms > 0 => "(newer)",
                Some(ms) if ms < 0 => "(older)",
                Some(_) => "(simultaneous)",
                None => "(overflow)",
            }
        );

        // Create classified entanglement bond if strength is significant
        if entanglement_strength > 0.7 {
            let entanglement_type = if let Some(temp_dist) = temporal_distance_ms {
                let abs_dist = temp_dist.abs();

                if abs_dist < 1000 {
                    EntanglementType::Causal
                } else if abs_dist < 60_000 {
                    EntanglementType::Temporal
                } else if entanglement_strength > 0.95 {
                    EntanglementType::BellPair
                } else if entanglement_strength > 0.85 {
                    EntanglementType::Bell
                } else {
                    EntanglementType::Semantic
                }
            } else if entanglement_strength > 0.95 {
                EntanglementType::BellPair
            } else if entanglement_strength > 0.85 {
                EntanglementType::Bell
            } else {
                EntanglementType::Semantic
            };

            let related_id_uuid = match uuid::Uuid::parse_str(&related_memory.id) {
                Ok(uuid) => uuid,
                Err(e) => {
                    log::warn!(
                        "Failed to parse UUID for memory {}: {}",
                        related_memory.id,
                        e
                    );
                    continue;
                }
            };

            if let Some(temporal_dist) = temporal_distance_ms
                && let Ok(source_uuid) = uuid::Uuid::parse_str(&memory.id)
            {
                let causal_link = CausalLink::new(
                    source_uuid,
                    related_id_uuid,
                    entanglement_strength,
                    temporal_dist,
                );

                log::info!(
                    "Causal link: {} -> {} (strength: {:.3}, temporal: {}ms, direction: {})",
                    memory.id,
                    related_memory.id,
                    causal_link.strength,
                    causal_link.temporal_distance,
                    if temporal_dist > 0 {
                        "forward"
                    } else if temporal_dist < 0 {
                        "backward"
                    } else {
                        "simultaneous"
                    }
                );

                // Store the causal link in database
                if let Err(e) = manager
                    .create_causal_edge(
                        &memory.id,
                        &related_memory.id,
                        causal_link.strength,
                        causal_link.temporal_distance,
                    )
                    .await
                {
                    log::error!(
                        "Failed to create causal edge {} -> {}: {:?}",
                        memory.id,
                        related_memory.id,
                        e
                    );
                } else {
                    log::debug!(
                        "Stored causal edge: {} ->caused-> {} (strength: {:.3}, temporal: {}ms)",
                        memory.id,
                        related_memory.id,
                        causal_link.strength,
                        causal_link.temporal_distance
                    );
                }
            }

            let bond_created = state_a
                .add_quantum_entanglement_bond(
                    related_id_uuid,
                    entanglement_strength,
                    entanglement_type,
                )
                .await;

            if bond_created {
                log::info!(
                    "Created {:?} entanglement bond: {} <-> {} (strength: {:.4}, temporal_dist: {:?}ms)",
                    entanglement_type,
                    memory.id,
                    related_memory.id,
                    entanglement_strength,
                    temporal_distance_ms
                );

                if let Err(e) = manager
                    .create_entanglement_edge(
                        &memory.id,
                        &related_memory.id,
                        entanglement_type,
                        entanglement_strength,
                    )
                    .await
                {
                    log::error!(
                        "Failed to create entanglement edge {} -> {}: {:?}",
                        memory.id,
                        related_memory.id,
                        e
                    );
                } else {
                    log::debug!(
                        "Created entanglement edge: {} ->entangled-> {} (strength: {:.3}, type: {:?})",
                        memory.id,
                        related_memory.id,
                        entanglement_strength,
                        entanglement_type
                    );

                    if let Err(e) = manager
                        .update_quantum_signature(&memory.id, state_a.clone())
                        .await
                    {
                        log::error!(
                            "Failed to persist quantum signature for {}: {:?}",
                            memory.id,
                            e
                        );
                    } else {
                        let bond_count = state_a.quantum_entanglement_bond_count().await;
                        log::debug!(
                            "Persisted quantum signature for {} (bond count: {})",
                            memory.id,
                            bond_count
                        );
                    }

                    entangled_count += 1;
                }
            } else {
                log::warn!(
                    "Failed to create entanglement bond to {} (invalid strength or target)",
                    related_memory.id
                );
            }
        }
    }

    if entangled_count > 0 {
        if let Err(e) = manager
            .update_quantum_signature(&memory_id, state_a.clone())
            .await
        {
            log::error!(
                "Failed to persist final quantum signature for {}: {:?}",
                memory_id,
                e
            );
        } else {
            let bond_count = state_a.quantum_entanglement_bond_count().await;
            log::info!(
                "Persisted quantum signature for {} with {} entanglement bonds (cached)",
                memory_id,
                bond_count
            );
        }
    }

    log::info!(
        "Entanglement discovery complete for {}: {} links created in RELATION table",
        memory_id,
        entangled_count
    );
    tracker.complete_operation(op_id);
}
