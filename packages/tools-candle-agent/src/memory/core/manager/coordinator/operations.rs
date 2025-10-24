//! Core CRUD operations for memory management

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::domain::memory::primitives::node::MemoryNode;
use crate::memory::MemoryMetadata;
use crate::memory::core::cognitive_queue::{CognitiveTask, CognitiveTaskType};
use crate::memory::core::manager::surreal::trait_def::MemoryManager;
use crate::memory::utils::Result;

use super::lifecycle::MemoryCoordinator;
use super::types::LazyEvalStrategy;

impl MemoryCoordinator {
    /// Add a new memory to storage with deduplication and cognitive processing
    ///
    /// This method:
    /// 1. Checks for duplicate content using content hash
    /// 2. Refreshes existing memory if duplicate found
    /// 3. Stores new memory if unique
    /// 4. Queues background cognitive evaluation
    ///
    /// # Arguments
    /// * `content` - Text content of the memory
    /// * `memory_type` - Type classification of the memory
    /// * `metadata` - Optional metadata (user_id, agent_id, etc.)
    ///
    /// # Returns
    /// The created or refreshed memory node
    pub async fn add_memory(
        &self,
        content: String,
        memory_type: crate::domain::memory::primitives::types::MemoryTypeEnum,
        metadata: Option<MemoryMetadata>,
    ) -> Result<MemoryNode> {
        use crate::domain::memory::primitives::types::MemoryContent;

        // Calculate content hash for deduplication
        let content_hash = crate::domain::memory::serialization::content_hash(&content);

        // Check if document with same content hash already exists
        if let Some(existing_memory) = self
            .surreal_manager
            .find_document_by_hash(content_hash)
            .await?
        {
            // Found duplicate! Refresh its age instead of re-ingesting
            log::info!(
                "Duplicate content detected (hash: {}), refreshing existing memory {}",
                content_hash,
                existing_memory.id
            );

            // Convert to domain node
            let mut domain_memory = self.convert_memory_to_domain_node(&existing_memory)?;

            // Refresh last_accessed_at to mark as recently seen
            domain_memory.stats.record_read();

            // Update importance to reflect re-occurrence (boost by 10%)
            let current_importance = domain_memory.importance();
            let boosted_importance = (current_importance * 1.1).min(1.0);
            domain_memory
                .set_importance(boosted_importance)
                .map_err(|e| crate::memory::utils::Error::Internal(format!("{:?}", e)))?;

            // Convert back and persist the refresh
            let memory_node = self.convert_domain_to_memory_node(&domain_memory);
            self.surreal_manager
                .update_memory(memory_node.clone())
                .await?;

            log::trace!(
                "Refreshed existing memory: importance {} -> {}",
                current_importance,
                boosted_importance
            );

            return Ok(domain_memory);
        }

        // Create new domain memory node
        let memory_content = MemoryContent::text(&content);
        let mut domain_memory = MemoryNode::new(memory_type, memory_content);

        // Apply metadata if provided
        if let Some(ref metadata) = metadata {
            // Import user_id, agent_id, context into custom metadata
            let mut custom_map = std::collections::HashMap::new();

            if let Some(ref user_id) = metadata.user_id {
                custom_map.insert(
                    Arc::from("user_id"),
                    Arc::new(serde_json::Value::String(user_id.clone())),
                );
            }

            if let Some(ref agent_id) = metadata.agent_id {
                custom_map.insert(
                    Arc::from("agent_id"),
                    Arc::new(serde_json::Value::String(agent_id.clone())),
                );
            }

            custom_map.insert(
                Arc::from("context"),
                Arc::new(serde_json::Value::String(metadata.context.clone())),
            );

            // Apply metadata
            domain_memory.metadata = Arc::new(
                crate::domain::memory::primitives::node::MemoryNodeMetadata {
                    importance: metadata.importance,
                    keywords: metadata
                        .keywords
                        .iter()
                        .map(|s| Arc::from(s.as_str()))
                        .collect(),
                    tags: metadata
                        .tags
                        .iter()
                        .map(|s| Arc::from(s.as_str()))
                        .collect(),
                    custom: custom_map,
                    version: 1,
                },
            );
        }

        // Generate embedding using BERT
        let embedding = self.generate_embedding(&content).await?;
        domain_memory.embedding =
            Some(crate::domain::memory::primitives::node::AlignedEmbedding::new(embedding));

        // Automatic image embedding if metadata contains image_path
        if let Some(metadata) = &metadata
            && let Some(image_path_value) = metadata.custom.get("image_path")
            && let Some(image_path) = image_path_value.as_str()
        {
            use crate::capability::registry;
            use crate::capability::traits::ImageEmbeddingCapable;

            // Query registry for image embedding model (graceful degradation if unavailable)
            if let Some(vision_model) = registry::get::<
                crate::capability::registry::ImageEmbeddingModel,
            >("openai/clip-vit-base-patch32")
            {
                match vision_model.embed_image(image_path).await {
                    Ok(image_embedding) => {
                        match self
                            .cognitive_state
                            .write()
                            .await
                            .update_activation_from_stimulus(image_embedding)
                        {
                            Ok(()) => {
                                log::trace!(
                                    "Updated cognitive activation from image: {}",
                                    image_path
                                );
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to update cognitive activation from image: {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to generate image embedding for {}: {}",
                            image_path,
                            e
                        );
                    }
                }
            } else {
                log::debug!(
                    "Image embedding model not available in registry, skipping image processing for: {}",
                    image_path
                );
            }
        }

        // Convert to core memory node for storage
        let memory_node = self.convert_domain_to_memory_node(&domain_memory);

        // Store in SurrealDB
        let stored_memory = self.surreal_manager.create_memory(memory_node).await?;

        // Add to in-memory repository cache
        {
            let mut repo = self.repository.write().await;
            repo.add(stored_memory.clone());
        }

        // Queue for cognitive evaluation
        let task = CognitiveTask::new(
            stored_memory.id.clone(),
            CognitiveTaskType::CommitteeEvaluation,
            5, // Default priority
        );
        self.cognitive_queue
            .enqueue(task)
            .map_err(crate::memory::utils::Error::Internal)?;

        // Convert stored memory back to domain format for return
        let final_domain_memory = self.convert_memory_to_domain_node(&stored_memory)?;

        Ok(final_domain_memory)
    }

    /// Retrieve a memory by ID with lazy evaluation support
    ///
    /// Supports three evaluation strategies:
    /// - `WaitForCompletion`: Polls until cognitive processing finishes
    /// - `ReturnPartial`: Returns immediately with available data (default)
    /// - `TriggerAndWait`: Bypasses queue and evaluates synchronously
    pub async fn get_memory(&self, memory_id: &str) -> Result<Option<MemoryNode>> {
        // Record long-term memory access
        self.cognitive_state
            .read()
            .await
            .stats()
            .record_long_term_memory_access();

        // Retrieve from SurrealDB
        let memory_node = match self.surreal_manager.get_memory(memory_id).await? {
            Some(node) => node,
            None => return Ok(None),
        };

        // Convert to domain node
        let domain_memory = self.convert_memory_to_domain_node(&memory_node)?;

        // Generate stimulus from memory embedding and update cognitive state
        if let Some(ref embedding) = domain_memory.embedding {
            let stimulus = embedding.data.clone();
            match self
                .cognitive_state
                .write()
                .await
                .update_activation_from_stimulus(stimulus)
            {
                Ok(()) => {
                    log::trace!(
                        "Updated cognitive activation from memory retrieval: {}",
                        memory_id
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Failed to update cognitive activation from memory retrieval: {}",
                        e
                    );
                }
            }
        }

        // Automatic image embedding processing on retrieval
        if let Some(image_path_value) = domain_memory.metadata.custom.get("image_path")
            && let Some(image_path) = image_path_value.as_ref().as_str()
        {
            use crate::capability::registry;
            use crate::capability::traits::ImageEmbeddingCapable;

            // Query registry for image embedding model (graceful degradation if unavailable)
            if let Some(vision_model) = registry::get::<
                crate::capability::registry::ImageEmbeddingModel,
            >("openai/clip-vit-base-patch32")
            {
                match vision_model.embed_image(image_path).await {
                    Ok(image_embedding) => {
                        match self
                            .cognitive_state
                            .write()
                            .await
                            .update_activation_from_stimulus(image_embedding)
                        {
                            Ok(()) => {
                                log::trace!(
                                    "Updated cognitive activation from retrieved image: {}",
                                    image_path
                                );
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to update cognitive activation from retrieved image: {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to generate image embedding on retrieval: {}", e);
                    }
                }
            } else {
                log::debug!(
                    "Image embedding model not available in registry, skipping image processing for: {}",
                    image_path
                );
            }
        }

        // NOTE: Temporal decay now applied by background DecayWorker
        // Removed lazy evaluation from read path for performance

        // Handle lazy evaluation based on strategy
        let evaluation_status = memory_node.evaluation_status;

        match self.lazy_eval_strategy {
            LazyEvalStrategy::ReturnPartial => {
                // Default: Return immediately, even if evaluation pending
                log::trace!(
                    "ReturnPartial: Returning memory {} with status {:?}",
                    memory_id,
                    evaluation_status
                );
                Ok(Some(domain_memory))
            }
            LazyEvalStrategy::WaitForCompletion => {
                // Poll until evaluation completes or timeout
                if matches!(
                    evaluation_status,
                    crate::memory::monitoring::operations::OperationStatus::Pending
                ) {
                    log::trace!(
                        "WaitForCompletion: Polling for memory {} evaluation",
                        memory_id
                    );

                    let start = Instant::now();
                    let timeout = Duration::from_secs(5);

                    loop {
                        if start.elapsed() > timeout {
                            log::warn!(
                                "Evaluation timeout for memory {}, returning partial",
                                memory_id
                            );
                            break;
                        }

                        // Check evaluation status from cache
                        if let Some(_score) = self.evaluation_cache.get(memory_id) {
                            log::trace!("Evaluation complete for memory {}", memory_id);
                            break;
                        }

                        // Small delay before next check
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }

                Ok(Some(domain_memory))
            }
            LazyEvalStrategy::TriggerAndWait => {
                // Bypass queue and evaluate synchronously
                if matches!(
                    evaluation_status,
                    crate::memory::monitoring::operations::OperationStatus::Pending
                ) {
                    log::trace!(
                        "TriggerAndWait: Immediate evaluation for memory {}",
                        memory_id
                    );

                    // Check cache first
                    if let Some(score) = self.evaluation_cache.get(memory_id) {
                        log::trace!("Using cached evaluation score: {}", score);
                    } else {
                        // Perform immediate evaluation
                        let score = self
                            .committee_evaluator
                            .evaluate(&domain_memory.content().to_string())
                            .await
                            .map_err(|e| {
                                crate::memory::utils::Error::Internal(format!(
                                    "Committee evaluation failed: {:?}",
                                    e
                                ))
                            })?;

                        // Cache the result
                        self.evaluation_cache.insert(memory_id.to_string(), score);

                        log::trace!("Immediate evaluation complete: score = {}", score);
                    }
                }

                Ok(Some(domain_memory))
            }
        }
    }

    /// Update an existing memory
    pub async fn update_memory(&self, memory: MemoryNode) -> Result<MemoryNode> {
        // Convert to core memory node
        let memory_node = self.convert_domain_to_memory_node(&memory);

        // Update in SurrealDB
        let updated_memory = self.surreal_manager.update_memory(memory_node).await?;

        // Update in-memory repository
        {
            let mut repo = self.repository.write().await;
            repo.update(updated_memory.clone());
        }

        // Convert back to domain format
        let final_domain_memory = self.convert_memory_to_domain_node(&updated_memory)?;

        Ok(final_domain_memory)
    }

    /// Delete a memory by ID
    pub async fn delete_memory(&self, memory_id: &str) -> Result<()> {
        // Delete from SurrealDB
        self.surreal_manager.delete_memory(memory_id).await?;

        // Remove from in-memory repository
        {
            let mut repo = self.repository.write().await;
            repo.delete(memory_id);
        }

        log::info!("Deleted memory: {}", memory_id);

        Ok(())
    }

    /// Get cognitive performance statistics
    ///
    /// Returns atomic counters for cognitive operations. All counters are
    /// thread-safe and can be read without blocking.
    pub async fn cognitive_stats(
        &self,
    ) -> Arc<crate::domain::memory::cognitive::types::CognitiveStats> {
        let state = self.cognitive_state.read().await;
        state.stats_arc()
    }
}
