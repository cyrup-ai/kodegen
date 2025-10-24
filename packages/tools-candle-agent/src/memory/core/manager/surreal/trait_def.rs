//! MemoryManager trait definition for memory operations.
//!
//! This module defines the core trait for memory management systems,
//! providing a unified interface for CRUD operations, search, and
//! quantum entanglement features.

use crate::domain::memory::cognitive::types::{CognitiveState, EntanglementType};
use crate::memory::primitives::{MemoryNode, MemoryRelationship};

use super::futures::{
    MemoryQuery, MemoryStream, PendingDeletion, PendingEntanglementEdge, PendingMemory,
    PendingQuantumSignature, PendingQuantumUpdate, PendingRelationship, RelationshipStream,
};

/// Core memory management trait defining operations for storing, retrieving, and managing memory nodes
pub trait MemoryManager: Send + Sync {
    // === Core Memory CRUD Operations ===

    /// Create a new memory node
    fn create_memory(&self, memory: MemoryNode) -> PendingMemory;

    /// Get a memory node by ID
    fn get_memory(&self, id: &str) -> MemoryQuery;

    /// Update an existing memory node
    fn update_memory(&self, memory: MemoryNode) -> PendingMemory;

    /// Delete a memory node by ID
    fn delete_memory(&self, id: &str) -> PendingDeletion;

    // === Search and Query Operations ===

    /// Search memories by embedding vector similarity
    fn search_by_vector(&self, vector: Vec<f32>, limit: usize) -> MemoryStream;

    /// Search memories by content text
    fn search_by_content(&self, text: &str) -> MemoryStream;

    /// Query memories by type
    fn query_by_type(
        &self,
        memory_type: crate::memory::primitives::types::MemoryTypeEnum,
    ) -> MemoryStream;

    /// List all memories with pagination support
    fn list_all_memories(&self, limit: usize, offset: usize) -> MemoryStream;

    // === Relationship Operations ===

    /// Create a relationship between two memories
    fn create_relationship(&self, relationship: MemoryRelationship) -> PendingRelationship;

    /// Get all relationships for a memory node
    fn get_relationships(&self, memory_id: &str) -> RelationshipStream;

    /// Delete a relationship by ID
    fn delete_relationship(&self, id: &str) -> PendingDeletion;

    // === Quantum Memory Operations ===

    /// Update quantum signature for a memory node
    fn update_quantum_signature(
        &self,
        memory_id: &str,
        signature: CognitiveState,
    ) -> PendingQuantumUpdate;

    /// Get quantum signature for a memory node
    fn get_quantum_signature(&self, memory_id: &str) -> PendingQuantumSignature;

    // === Entanglement Operations ===

    /// Create an entanglement edge between two memories
    fn create_entanglement_edge(
        &self,
        source_id: &str,
        target_id: &str,
        entanglement_type: EntanglementType,
        strength: f32,
    ) -> PendingEntanglementEdge;

    /// Create a causal edge between two memories
    fn create_causal_edge(
        &self,
        source_id: &str,
        target_id: &str,
        strength: f32,
        temporal_distance: i64,
    ) -> PendingEntanglementEdge;

    /// Get all entangled memories for a given memory ID
    fn get_entangled_memories(&self, memory_id: &str) -> MemoryStream;

    /// Get entangled memories filtered by entanglement type
    fn get_entangled_by_type(
        &self,
        memory_id: &str,
        entanglement_type: EntanglementType,
    ) -> MemoryStream;

    /// Traverse entanglement graph from a starting memory
    fn traverse_entanglement_graph(
        &self,
        start_memory_id: &str,
        max_depth: usize,
        min_strength: f32,
    ) -> MemoryStream;

    /// Expand search results via entanglement relationships
    fn expand_via_entanglement(
        &self,
        seed_memory_ids: Vec<String>,
        expansion_factor: usize,
        min_strength: f32,
    ) -> MemoryStream;

    // === Causal Reasoning Operations ===

    /// Get memories that causally preceded this one (what caused this?)
    fn get_causal_predecessors(&self, memory_id: &str) -> MemoryStream;

    /// Get memories that this causally influenced (what did this cause?)
    fn get_causal_successors(&self, memory_id: &str) -> MemoryStream;

    /// Traverse causal chain forward from a memory
    fn trace_causal_chain_forward(&self, start_memory_id: &str, max_depth: usize) -> MemoryStream;

    /// Traverse causal chain backward to find root causes
    fn trace_causal_chain_backward(&self, start_memory_id: &str, max_depth: usize) -> MemoryStream;
}

// Blanket implementation for Arc<T> to enable trait methods on Arc-wrapped managers
impl<T: MemoryManager + ?Sized> MemoryManager for std::sync::Arc<T> {
    fn create_memory(&self, memory: MemoryNode) -> PendingMemory {
        (**self).create_memory(memory)
    }

    fn get_memory(&self, id: &str) -> MemoryQuery {
        (**self).get_memory(id)
    }

    fn update_memory(&self, memory: MemoryNode) -> PendingMemory {
        (**self).update_memory(memory)
    }

    fn delete_memory(&self, id: &str) -> PendingDeletion {
        (**self).delete_memory(id)
    }

    fn search_by_vector(&self, vector: Vec<f32>, limit: usize) -> MemoryStream {
        (**self).search_by_vector(vector, limit)
    }

    fn search_by_content(&self, text: &str) -> MemoryStream {
        (**self).search_by_content(text)
    }

    fn query_by_type(
        &self,
        memory_type: crate::memory::primitives::types::MemoryTypeEnum,
    ) -> MemoryStream {
        (**self).query_by_type(memory_type)
    }

    fn list_all_memories(&self, limit: usize, offset: usize) -> MemoryStream {
        (**self).list_all_memories(limit, offset)
    }

    fn create_relationship(&self, relationship: MemoryRelationship) -> PendingRelationship {
        (**self).create_relationship(relationship)
    }

    fn get_relationships(&self, memory_id: &str) -> RelationshipStream {
        (**self).get_relationships(memory_id)
    }

    fn delete_relationship(&self, id: &str) -> PendingDeletion {
        (**self).delete_relationship(id)
    }

    fn update_quantum_signature(
        &self,
        memory_id: &str,
        signature: CognitiveState,
    ) -> PendingQuantumUpdate {
        (**self).update_quantum_signature(memory_id, signature)
    }

    fn get_quantum_signature(&self, memory_id: &str) -> PendingQuantumSignature {
        (**self).get_quantum_signature(memory_id)
    }

    fn create_entanglement_edge(
        &self,
        source_id: &str,
        target_id: &str,
        entanglement_type: EntanglementType,
        strength: f32,
    ) -> PendingEntanglementEdge {
        (**self).create_entanglement_edge(source_id, target_id, entanglement_type, strength)
    }

    fn create_causal_edge(
        &self,
        source_id: &str,
        target_id: &str,
        strength: f32,
        temporal_distance: i64,
    ) -> PendingEntanglementEdge {
        (**self).create_causal_edge(source_id, target_id, strength, temporal_distance)
    }

    fn get_entangled_memories(&self, memory_id: &str) -> MemoryStream {
        (**self).get_entangled_memories(memory_id)
    }

    fn get_entangled_by_type(
        &self,
        memory_id: &str,
        entanglement_type: EntanglementType,
    ) -> MemoryStream {
        (**self).get_entangled_by_type(memory_id, entanglement_type)
    }

    fn traverse_entanglement_graph(
        &self,
        start_memory_id: &str,
        max_depth: usize,
        min_strength: f32,
    ) -> MemoryStream {
        (**self).traverse_entanglement_graph(start_memory_id, max_depth, min_strength)
    }

    fn expand_via_entanglement(
        &self,
        seed_memory_ids: Vec<String>,
        expansion_factor: usize,
        min_strength: f32,
    ) -> MemoryStream {
        (**self).expand_via_entanglement(seed_memory_ids, expansion_factor, min_strength)
    }

    fn get_causal_predecessors(&self, memory_id: &str) -> MemoryStream {
        (**self).get_causal_predecessors(memory_id)
    }

    fn get_causal_successors(&self, memory_id: &str) -> MemoryStream {
        (**self).get_causal_successors(memory_id)
    }

    fn trace_causal_chain_forward(&self, start_memory_id: &str, max_depth: usize) -> MemoryStream {
        (**self).trace_causal_chain_forward(start_memory_id, max_depth)
    }

    fn trace_causal_chain_backward(&self, start_memory_id: &str, max_depth: usize) -> MemoryStream {
        (**self).trace_causal_chain_backward(start_memory_id, max_depth)
    }
}
