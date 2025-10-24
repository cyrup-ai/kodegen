//! Type conversion utilities between domain and core layers

use std::sync::Arc;

use crate::memory::utils::{Error, Result};

use super::lifecycle::MemoryCoordinator;

impl MemoryCoordinator {
    /// Convert domain MemoryNode to memory MemoryNode for storage compatibility
    pub(in crate::memory::core) fn convert_domain_to_memory_node(
        &self,
        domain_node: &crate::domain::memory::primitives::node::MemoryNode,
    ) -> crate::memory::core::primitives::node::MemoryNode {
        use crate::memory::core::primitives::{
            metadata::MemoryMetadata as MemoryMemoryMetadata, node::MemoryNode as MemoryMemoryNode,
            types::MemoryContent as MemoryMemoryContent,
            types::MemoryTypeEnum as MemoryMemoryTypeEnum,
        };

        let embedding_vec = domain_node
            .embedding()
            .map(|aligned_emb| aligned_emb.to_vec());

        // Create memory system metadata preserving domain metadata
        let memory_metadata = MemoryMemoryMetadata {
            user_id: domain_node
                .metadata
                .custom
                .get("user_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            agent_id: domain_node
                .metadata
                .custom
                .get("agent_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            context: domain_node
                .metadata
                .custom
                .get("context")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string(),
            importance: domain_node.importance(),
            keywords: domain_node
                .metadata
                .keywords
                .iter()
                .map(|k| k.to_string())
                .collect(),
            tags: domain_node
                .metadata
                .tags
                .iter()
                .map(|t| t.to_string())
                .collect(),
            category: domain_node
                .metadata
                .custom
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("general")
                .to_string(),
            source: domain_node
                .metadata
                .custom
                .get("source")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            created_at: domain_node.base_memory.created_at.into(),
            last_accessed_at: Some(chrono::DateTime::<chrono::Utc>::from(
                domain_node.last_accessed(),
            )),
            embedding: embedding_vec.clone(),
            custom: serde_json::to_value(&domain_node.metadata.custom)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new())),
        };

        // Create memory content
        let memory_content = MemoryMemoryContent::new(&domain_node.content().to_string());

        // Convert memory type - map to closest equivalent
        let memory_type = match domain_node.memory_type() {
            crate::domain::memory::primitives::types::MemoryTypeEnum::Semantic => {
                MemoryMemoryTypeEnum::Semantic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Episodic => {
                MemoryMemoryTypeEnum::Episodic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Procedural => {
                MemoryMemoryTypeEnum::Procedural
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Working => {
                MemoryMemoryTypeEnum::Working
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::LongTerm => {
                MemoryMemoryTypeEnum::LongTerm
            }
            // Map additional domain variants to closest memory system equivalents
            crate::domain::memory::primitives::types::MemoryTypeEnum::Fact => {
                MemoryMemoryTypeEnum::Semantic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Episode => {
                MemoryMemoryTypeEnum::Episodic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Declarative => {
                MemoryMemoryTypeEnum::Semantic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Implicit => {
                MemoryMemoryTypeEnum::Procedural
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Explicit => {
                MemoryMemoryTypeEnum::Semantic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Contextual => {
                MemoryMemoryTypeEnum::Semantic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Temporal => {
                MemoryMemoryTypeEnum::Episodic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Spatial => {
                MemoryMemoryTypeEnum::Episodic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Associative => {
                MemoryMemoryTypeEnum::Semantic
            }
            crate::domain::memory::primitives::types::MemoryTypeEnum::Emotional => {
                MemoryMemoryTypeEnum::Episodic
            }
        };

        // Calculate content hash
        let content_hash = crate::domain::memory::serialization::content_hash(&memory_content.text);

        MemoryMemoryNode {
            id: domain_node.id().to_string(),
            content: memory_content,
            content_hash,
            memory_type,
            created_at: domain_node.base_memory.created_at.into(),
            updated_at: domain_node.base_memory.updated_at.into(),
            embedding: embedding_vec,
            evaluation_status: crate::memory::monitoring::operations::OperationStatus::Pending,
            metadata: memory_metadata,
            relevance_score: None, // No score for stored memories, only for retrieved ones
        }
    }

    /// Convert memory MemoryNode to domain MemoryNode for API compatibility
    pub(in crate::memory::core) fn convert_memory_to_domain_node(
        &self,
        memory_node: &crate::memory::core::primitives::node::MemoryNode,
    ) -> Result<crate::domain::memory::primitives::node::MemoryNode> {
        use crate::domain::memory::primitives::{
            node::{AlignedEmbedding, MemoryNode as DomainMemoryNode, MemoryNodeMetadata},
            types::{MemoryContent as DomainMemoryContent, MemoryTypeEnum as DomainMemoryTypeEnum},
        };
        use uuid::Uuid;

        // Convert memory type - map to closest equivalent
        let domain_memory_type = match memory_node.memory_type {
            crate::memory::core::primitives::types::MemoryTypeEnum::Semantic => {
                DomainMemoryTypeEnum::Semantic
            }
            crate::memory::core::primitives::types::MemoryTypeEnum::Episodic => {
                DomainMemoryTypeEnum::Episodic
            }
            crate::memory::core::primitives::types::MemoryTypeEnum::Procedural => {
                DomainMemoryTypeEnum::Procedural
            }
            crate::memory::core::primitives::types::MemoryTypeEnum::Working => {
                DomainMemoryTypeEnum::Working
            }
            crate::memory::core::primitives::types::MemoryTypeEnum::LongTerm => {
                DomainMemoryTypeEnum::LongTerm
            }
        };

        // Create domain content
        let domain_content = DomainMemoryContent::text(&memory_node.content.text);

        // Parse UUID from string ID - fail fast on corruption
        let uuid = Uuid::parse_str(&memory_node.id).map_err(|e| {
            Error::Internal(format!(
                "Invalid UUID in memory node {}: {}",
                memory_node.id, e
            ))
        })?;

        // Create domain node
        let mut domain_node = DomainMemoryNode::with_id(uuid, domain_memory_type, domain_content);

        // Convert embedding if present
        if let Some(embedding_vec) = &memory_node.embedding {
            domain_node.embedding = Some(AlignedEmbedding::new(embedding_vec.clone()));
        }

        // Convert metadata, preserving evaluation_status in custom field
        let mut custom_map: std::collections::HashMap<Arc<str>, Arc<serde_json::Value>> =
            memory_node
                .metadata
                .custom
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (Arc::from(k.as_str()), Arc::new(v.clone())))
                        .collect()
                })
                .unwrap_or_default();

        // Store evaluation_status in custom metadata for domain layer
        custom_map.insert(
            Arc::from("evaluation_status"),
            Arc::new(
                serde_json::to_value(&memory_node.evaluation_status)
                    .unwrap_or(serde_json::Value::Null),
            ),
        );

        let domain_metadata = MemoryNodeMetadata {
            importance: memory_node.metadata.importance,
            keywords: memory_node
                .metadata
                .keywords
                .iter()
                .map(|k| k.clone().into())
                .collect(),
            tags: memory_node
                .metadata
                .tags
                .iter()
                .map(|t| t.clone().into())
                .collect(),
            custom: custom_map,
            version: 1,
        };
        domain_node.metadata = std::sync::Arc::new(domain_metadata);

        Ok(domain_node)
    }

    /// Generate embedding for text content using configured embedding model
    pub(super) async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        use crate::capability::traits::TextEmbeddingCapable;

        // Use configured embedding provider (default: Stella 1024)
        let embedding = self
            .embedding_model
            .embed(text, None)
            .await
            .map_err(|e| Error::Internal(format!("Embedding generation failed: {}", e)))?;
        Ok(embedding)
    }
}
