//! MemoryManager trait implementation for SurrealDB.
//!
//! This module contains the complete implementation of the MemoryManager trait
//! for SurrealDBMemoryManager, providing all CRUD operations, search, and
//! quantum entanglement features.

use crate::capability::traits::TextEmbeddingCapable;
use crate::domain::memory::cognitive::types::{CognitiveState, EntanglementType};
use crate::memory::core::primitives::types::MemoryTypeEnum;
use crate::memory::primitives::{MemoryNode, MemoryRelationship};
use crate::memory::schema::memory_schema::MemoryNodeSchema;
use crate::memory::schema::quantum_schema::QuantumSignatureSchema;
use crate::memory::schema::relationship_schema::Relationship;
use crate::memory::utils::error::Error;

use super::futures::{
    MemoryQuery, MemoryStream, PendingDeletion, PendingEntanglementEdge, PendingMemory,
    PendingQuantumSignature, PendingQuantumUpdate, PendingRelationship, RelationshipStream,
};
use super::manager::SurrealDBMemoryManager;
use super::trait_def::MemoryManager;
use super::types::{MemoryNodeCreateContent, RelationshipCreateContent};

impl MemoryManager for SurrealDBMemoryManager {
    fn create_memory(&self, memory: MemoryNode) -> PendingMemory {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let embedding_model = self.embedding_model.clone();

        tokio::spawn(async move {
            let result = async {
                let mut memory_with_embedding = memory.clone();

                // Auto-generate embedding if model is configured and embedding is missing
                if let Some(ref model) = embedding_model
                    && memory.metadata.embedding.is_none()
                {
                    log::info!("Auto-generating embedding for memory: {}", memory.id);
                    let embedding = model
                        .embed(&memory.content.text, Some("document".to_string()))
                        .await?;
                    memory_with_embedding.metadata.embedding = Some(embedding);
                }

                let content = MemoryNodeCreateContent::from(&memory_with_embedding);

                let query = "
                    CREATE memory CONTENT {
                        id: $id,
                        content: $content,
                        content_hash: $content_hash,
                        memory_type: $memory_type,
                        created_at: $created_at,
                        updated_at: $updated_at,
                        metadata: $metadata
                    }
                ";

                let mut response = db
                    .query(query)
                    .bind(("id", memory.id.clone()))
                    .bind(("content", content.content))
                    .bind(("content_hash", content.content_hash))
                    .bind(("memory_type", format!("{:?}", content.memory_type)))
                    .bind(("created_at", memory.created_at))
                    .bind(("updated_at", memory.updated_at))
                    .bind(("metadata", content.metadata))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                let result: Vec<MemoryNodeSchema> = response
                    .take(0)
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                result
                    .into_iter()
                    .next()
                    .map(SurrealDBMemoryManager::from_schema)
                    .ok_or_else(|| Error::Other("Failed to create memory".to_string()))
            }
            .await;

            let _ = tx.send(result);
        });

        PendingMemory::new(rx)
    }

    fn get_memory(&self, id: &str) -> MemoryQuery {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let id = id.to_string();

        tokio::spawn(async move {
            let result = async {
                let query = "SELECT * FROM $id";

                let mut response = db
                    .query(query)
                    .bind(("id", id))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                let results: Vec<MemoryNodeSchema> = response
                    .take(0)
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                Ok(results
                    .into_iter()
                    .next()
                    .map(SurrealDBMemoryManager::from_schema))
            }
            .await;

            let _ = tx.send(result);
        });

        MemoryQuery::new(rx)
    }

    fn update_memory(&self, memory: MemoryNode) -> PendingMemory {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();

        tokio::spawn(async move {
            let result = async {
                let content = MemoryNodeCreateContent::from(&memory);

                let query = "
                    UPDATE $id SET
                        content = $content,
                        content_hash = $content_hash,
                        memory_type = $memory_type,
                        updated_at = $updated_at,
                        metadata = $metadata
                ";

                let mut response = db
                    .query(query)
                    .bind(("id", memory.id.clone()))
                    .bind(("content", content.content))
                    .bind(("content_hash", content.content_hash))
                    .bind(("memory_type", format!("{:?}", content.memory_type)))
                    .bind(("updated_at", memory.updated_at))
                    .bind(("metadata", content.metadata))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                let result: Vec<MemoryNodeSchema> = response
                    .take(0)
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                result
                    .into_iter()
                    .next()
                    .map(SurrealDBMemoryManager::from_schema)
                    .ok_or_else(|| Error::Other("Failed to update memory".to_string()))
            }
            .await;

            let _ = tx.send(result);
        });

        PendingMemory::new(rx)
    }

    fn delete_memory(&self, id: &str) -> PendingDeletion {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let id = id.to_string();

        tokio::spawn(async move {
            let result = async {
                let query = "DELETE $id";

                let mut response = db
                    .query(query)
                    .bind(("id", id))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                let _result: Vec<serde_json::Value> = response
                    .take(0)
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                Ok(true)
            }
            .await;

            let _ = tx.send(result);
        });

        PendingDeletion::new(rx)
    }

    fn search_by_vector(&self, vector: Vec<f32>, limit: usize) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();

        tokio::spawn(async move {
            let vector_json = serde_json::to_string(&vector).unwrap_or_default();

            let query = format!(
                "SELECT *, 
                        vector::similarity::cosine(metadata.embedding, {}) AS vector_score
                 FROM memory
                 WHERE metadata.embedding != NULL
                 ORDER BY vector_score DESC
                 LIMIT {}",
                vector_json, limit
            );

            match db.query(&query).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn search_by_content(&self, text: &str) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let search_text = text.to_string();

        tokio::spawn(async move {
            let query = format!(
                "SELECT * FROM memory WHERE content CONTAINS \"{}\" ORDER BY metadata.created_at DESC LIMIT 100",
                search_text.replace("\"", "\\\"")
            );

            match db.query(&query).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn query_by_type(&self, memory_type: MemoryTypeEnum) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();

        tokio::spawn(async move {
            let type_str = format!("{:?}", memory_type);
            let query = format!(
                "SELECT * FROM memory WHERE memory_type = \"{}\" ORDER BY metadata.created_at DESC LIMIT 100",
                type_str
            );

            match db.query(&query).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn list_all_memories(&self, limit: usize, offset: usize) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();

        tokio::spawn(async move {
            let query = "SELECT * FROM memory START $offset LIMIT $limit ORDER BY created_at DESC";

            match db
                .query(query)
                .bind(("offset", offset))
                .bind(("limit", limit))
                .await
            {
                Ok(mut response) => match response.take::<Vec<MemoryNodeSchema>>(0) {
                    Ok(results) => {
                        for schema in results {
                            let memory = SurrealDBMemoryManager::from_schema(schema);
                            if tx.send(Ok(memory)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to parse memory batch (offset={}, limit={}): {:?}",
                            offset,
                            limit,
                            e
                        );
                        let _ = tx
                            .send(Err(Error::Database(format!(
                                "Memory batch parse failed: {:?}",
                                e
                            ))))
                            .await;
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn create_relationship(&self, relationship: MemoryRelationship) -> PendingRelationship {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();

        tokio::spawn(async move {
            let result = async {
                let content = RelationshipCreateContent::from(&relationship);

                let query = "
                    CREATE relationship CONTENT {
                        id: $id,
                        source_id: $source_id,
                        target_id: $target_id,
                        relationship_type: $relationship_type,
                        created_at: $created_at,
                        updated_at: $updated_at,
                        strength: $strength,
                        metadata: $metadata
                    }
                ";

                let mut response = db
                    .query(query)
                    .bind(("id", relationship.id.clone()))
                    .bind(("source_id", content.source_id))
                    .bind(("target_id", content.target_id))
                    .bind(("relationship_type", content.relationship_type))
                    .bind(("created_at", content.created_at))
                    .bind(("updated_at", content.updated_at))
                    .bind(("strength", content.strength))
                    .bind(("metadata", content.metadata))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                let result: Vec<Relationship> = response
                    .take(0)
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                result
                    .into_iter()
                    .next()
                    .map(|r| MemoryRelationship {
                        id: format!("relationship:{}", r.id),
                        source_id: r.source_id,
                        target_id: r.target_id,
                        relationship_type: r.relationship_type,
                        metadata: Some(r.metadata),
                        created_at: Some(r.created_at),
                        updated_at: Some(r.updated_at),
                        strength: Some(r.strength),
                    })
                    .ok_or_else(|| Error::Other("Failed to create relationship".to_string()))
            }
            .await;

            let _ = tx.send(result);
        });

        PendingRelationship::new(rx)
    }

    fn get_relationships(&self, memory_id: &str) -> RelationshipStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let memory_id = memory_id.to_string();

        tokio::spawn(async move {
            let query =
                "SELECT * FROM relationship WHERE source_id = $memory_id OR target_id = $memory_id";

            match db.query(query).bind(("memory_id", memory_id)).await {
                Ok(mut response) => {
                    let results: Vec<Relationship> = response.take(0).unwrap_or_default();

                    for r in results {
                        let relationship = MemoryRelationship {
                            id: format!("relationship:{}", r.id),
                            source_id: r.source_id,
                            target_id: r.target_id,
                            relationship_type: r.relationship_type,
                            metadata: Some(r.metadata),
                            created_at: Some(r.created_at),
                            updated_at: Some(r.updated_at),
                            strength: Some(r.strength),
                        };

                        if tx.send(Ok(relationship)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        RelationshipStream::new(rx)
    }

    fn delete_relationship(&self, id: &str) -> PendingDeletion {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let id = id.to_string();

        tokio::spawn(async move {
            let result = async {
                let query = "DELETE $id";

                let mut response = db
                    .query(query)
                    .bind(("id", id))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                let _result: Vec<serde_json::Value> = response
                    .take(0)
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                Ok(true)
            }
            .await;

            let _ = tx.send(result);
        });

        PendingDeletion::new(rx)
    }

    fn update_quantum_signature(
        &self,
        memory_id: &str,
        signature: CognitiveState,
    ) -> PendingQuantumUpdate {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let memory_id = memory_id.to_string();

        tokio::spawn(async move {
            let result = async {
                let schema = QuantumSignatureSchema::from_cognitive_state(&signature).await;

                let query = "
                    CREATE quantum_signature CONTENT {
                        memory_id: $memory_id,
                        coherence_fingerprint: $coherence_fingerprint,
                        entanglement_bonds: $entanglement_bonds,
                        superposition_contexts: $superposition_contexts,
                        collapse_probability: $collapse_probability,
                        quantum_entropy: $quantum_entropy,
                        created_at: $created_at,
                        decoherence_rate: $decoherence_rate
                    }
                ";

                db.query(query)
                    .bind(("memory_id", memory_id))
                    .bind(("coherence_fingerprint", schema.coherence_fingerprint))
                    .bind(("entanglement_bonds", schema.entanglement_bonds))
                    .bind(("superposition_contexts", schema.superposition_contexts))
                    .bind(("collapse_probability", schema.collapse_probability))
                    .bind(("quantum_entropy", schema.quantum_entropy))
                    .bind(("created_at", schema.created_at))
                    .bind(("decoherence_rate", schema.decoherence_rate))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                Ok(())
            }
            .await;

            let _ = tx.send(result);
        });

        PendingQuantumUpdate::new(rx)
    }

    fn get_quantum_signature(&self, memory_id: &str) -> PendingQuantumSignature {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let memory_id = memory_id.to_string();

        tokio::spawn(async move {
            let result = async {
                let query = "SELECT * FROM quantum_signature WHERE memory_id = $memory_id LIMIT 1";

                let mut response = db
                    .query(query)
                    .bind(("memory_id", memory_id))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                let results: Vec<QuantumSignatureSchema> = response
                    .take(0)
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                Ok(results
                    .into_iter()
                    .next()
                    .and_then(|schema| schema.to_cognitive_state().ok()))
            }
            .await;

            let _ = tx.send(result);
        });

        PendingQuantumSignature::new(rx)
    }

    fn create_entanglement_edge(
        &self,
        source_id: &str,
        target_id: &str,
        entanglement_type: EntanglementType,
        strength: f32,
    ) -> PendingEntanglementEdge {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let source_id = source_id.to_string();
        let target_id = target_id.to_string();

        tokio::spawn(async move {
            let result = async {
                let now = crate::memory::utils::current_timestamp_ms();
                let entanglement_type_str = format!("{:?}", entanglement_type);

                let query = format!(
                    "RELATE {}->entangled->{} SET entanglement_type = $entanglement_type, strength = $strength, created_at = $created_at",
                    source_id, target_id
                );

                db.query(&query)
                    .bind(("entanglement_type", entanglement_type_str))
                    .bind(("strength", strength))
                    .bind(("created_at", now))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                Ok(())
            }
            .await;

            let _ = tx.send(result);
        });

        PendingEntanglementEdge::new(rx)
    }

    fn create_causal_edge(
        &self,
        source_id: &str,
        target_id: &str,
        strength: f32,
        temporal_distance: i64,
    ) -> PendingEntanglementEdge {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let source_id = source_id.to_string();
        let target_id = target_id.to_string();

        tokio::spawn(async move {
            let result = async {
                let now = crate::memory::utils::current_timestamp_ms();

                let query = format!(
                    "RELATE {}->caused->{} SET strength = $strength, temporal_distance = $temporal_distance, created_at = $created_at",
                    source_id, target_id
                );

                db.query(&query)
                    .bind(("strength", strength))
                    .bind(("temporal_distance", temporal_distance))
                    .bind(("created_at", now))
                    .await
                    .map_err(|e| Error::Database(format!("{:?}", e)))?;

                Ok(())
            }
            .await;

            let _ = tx.send(result);
        });

        PendingEntanglementEdge::new(rx)
    }

    fn get_entangled_memories(&self, memory_id: &str) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let memory_id = memory_id.to_string();

        tokio::spawn(async move {
            let query = format!("SELECT out.* FROM {}->entangled", memory_id);

            match db.query(&query).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn get_entangled_by_type(
        &self,
        memory_id: &str,
        entanglement_type: EntanglementType,
    ) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let memory_id = memory_id.to_string();
        let type_str = format!("{:?}", entanglement_type);

        tokio::spawn(async move {
            let query = format!(
                "SELECT out.* FROM {}->entangled WHERE entanglement_type = $entanglement_type",
                memory_id
            );

            match db.query(&query).bind(("entanglement_type", type_str)).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn traverse_entanglement_graph(
        &self,
        start_memory_id: &str,
        max_depth: usize,
        min_strength: f32,
    ) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let start_id = start_memory_id.to_string();

        tokio::spawn(async move {
            let safe_depth = max_depth.min(5);

            let mut chain = String::from("->entangled");
            for _ in 1..safe_depth {
                chain.push_str("->memory->entangled");
            }

            let query = format!(
                "SELECT DISTINCT out.* FROM {}{} WHERE strength >= $min_strength",
                start_id, chain
            );

            match db.query(&query).bind(("min_strength", min_strength)).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn expand_via_entanglement(
        &self,
        seed_memory_ids: Vec<String>,
        expansion_factor: usize,
        min_strength: f32,
    ) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();

        tokio::spawn(async move {
            let ids_json = serde_json::to_string(&seed_memory_ids).unwrap_or_default();

            let query = format!(
                "SELECT DISTINCT out.*
                 FROM (SELECT VALUE id FROM {})
                 ->entangled
                 WHERE strength >= $min_strength
                 LIMIT {}",
                ids_json, expansion_factor
            );

            match db.query(&query).bind(("min_strength", min_strength)).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn get_causal_predecessors(&self, memory_id: &str) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let memory_id = memory_id.to_string();

        tokio::spawn(async move {
            // Query: SELECT in.* FROM memory_id<-caused
            // This gets all memories that caused this one
            let query = format!("SELECT in.* FROM {}<-caused", memory_id);

            match db.query(&query).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn get_causal_successors(&self, memory_id: &str) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let memory_id = memory_id.to_string();

        tokio::spawn(async move {
            // Query: SELECT out.* FROM memory_id->caused
            // This gets all memories that were caused by this one
            let query = format!("SELECT out.* FROM {}->caused", memory_id);

            match db.query(&query).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn trace_causal_chain_forward(&self, start_memory_id: &str, max_depth: usize) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let start_id = start_memory_id.to_string();

        tokio::spawn(async move {
            let safe_depth = max_depth.min(10); // Safety limit

            // Build chain: ->caused->memory->caused->memory...
            let mut chain = String::from("->caused");
            for _ in 1..safe_depth {
                chain.push_str("->memory->caused");
            }

            let query = format!("SELECT DISTINCT out.* FROM {}{}", start_id, chain);

            match db.query(&query).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }

    fn trace_causal_chain_backward(&self, start_memory_id: &str, max_depth: usize) -> MemoryStream {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let db = self.db.clone();
        let start_id = start_memory_id.to_string();

        tokio::spawn(async move {
            let safe_depth = max_depth.min(10); // Safety limit

            // Build chain: <-caused<-memory<-caused<-memory...
            let mut chain = String::from("<-caused");
            for _ in 1..safe_depth {
                chain.push_str("<-memory<-caused");
            }

            let query = format!("SELECT DISTINCT in.* FROM {}{}", start_id, chain);

            match db.query(&query).await {
                Ok(mut response) => {
                    let results: Vec<MemoryNodeSchema> = response.take(0).unwrap_or_default();

                    for schema in results {
                        let memory = SurrealDBMemoryManager::from_schema(schema);
                        if tx.send(Ok(memory)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Error::Database(format!("{:?}", e)))).await;
                }
            }
        });

        MemoryStream::new(rx)
    }
}
