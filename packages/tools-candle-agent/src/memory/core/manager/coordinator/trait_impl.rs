//! MemoryManager trait implementation for coordinator

use crate::domain::memory::cognitive::types::{CognitiveState, EntanglementType};
use crate::memory::MemoryRelationship;
use crate::memory::core::manager::surreal::{
    MemoryManager, MemoryStream, PendingDeletion, PendingEntanglementEdge, PendingMemory,
    PendingQuantumSignature, PendingQuantumUpdate, PendingRelationship, RelationshipStream,
};
use crate::memory::core::primitives::{
    node::MemoryNode as CoreMemoryNode, types::MemoryTypeEnum as CoreMemoryTypeEnum,
};

use super::lifecycle::MemoryCoordinator;

impl MemoryManager for MemoryCoordinator {
    // Delegate non-search methods directly to surreal_manager

    fn create_memory(&self, memory: CoreMemoryNode) -> PendingMemory {
        self.surreal_manager.create_memory(memory)
    }

    fn get_memory(&self, id: &str) -> crate::memory::core::manager::surreal::MemoryQuery {
        self.surreal_manager.get_memory(id)
    }

    fn update_memory(&self, memory: CoreMemoryNode) -> PendingMemory {
        self.surreal_manager.update_memory(memory)
    }

    fn delete_memory(&self, id: &str) -> PendingDeletion {
        self.surreal_manager.delete_memory(id)
    }

    fn create_relationship(&self, relationship: MemoryRelationship) -> PendingRelationship {
        self.surreal_manager.create_relationship(relationship)
    }

    fn get_relationships(&self, memory_id: &str) -> RelationshipStream {
        self.surreal_manager.get_relationships(memory_id)
    }

    fn delete_relationship(&self, id: &str) -> PendingDeletion {
        self.surreal_manager.delete_relationship(id)
    }

    fn search_by_vector(&self, vector: Vec<f32>, limit: usize) -> MemoryStream {
        // Vector search already uses quantum strategy by default
        self.surreal_manager.search_by_vector(vector, limit)
    }

    fn search_by_content(&self, text: &str) -> MemoryStream {
        self.surreal_manager.search_by_content(text)
    }

    fn query_by_type(&self, memory_type: CoreMemoryTypeEnum) -> MemoryStream {
        self.surreal_manager.query_by_type(memory_type)
    }

    fn list_all_memories(&self, limit: usize, offset: usize) -> MemoryStream {
        self.surreal_manager.list_all_memories(limit, offset)
    }

    fn update_quantum_signature(
        &self,
        memory_id: &str,
        signature: CognitiveState,
    ) -> PendingQuantumUpdate {
        self.surreal_manager
            .update_quantum_signature(memory_id, signature)
    }

    fn get_quantum_signature(&self, memory_id: &str) -> PendingQuantumSignature {
        self.surreal_manager.get_quantum_signature(memory_id)
    }

    fn create_entanglement_edge(
        &self,
        source_id: &str,
        target_id: &str,
        entanglement_type: EntanglementType,
        strength: f32,
    ) -> PendingEntanglementEdge {
        self.surreal_manager.create_entanglement_edge(
            source_id,
            target_id,
            entanglement_type,
            strength,
        )
    }

    fn get_entangled_memories(&self, memory_id: &str) -> MemoryStream {
        self.surreal_manager.get_entangled_memories(memory_id)
    }

    fn get_entangled_by_type(
        &self,
        memory_id: &str,
        entanglement_type: EntanglementType,
    ) -> MemoryStream {
        self.surreal_manager
            .get_entangled_by_type(memory_id, entanglement_type)
    }

    fn traverse_entanglement_graph(
        &self,
        start_memory_id: &str,
        max_depth: usize,
        min_strength: f32,
    ) -> MemoryStream {
        self.surreal_manager
            .traverse_entanglement_graph(start_memory_id, max_depth, min_strength)
    }

    fn expand_via_entanglement(
        &self,
        seed_memory_ids: Vec<String>,
        expansion_factor: usize,
        min_strength: f32,
    ) -> MemoryStream {
        self.surreal_manager.expand_via_entanglement(
            seed_memory_ids,
            expansion_factor,
            min_strength,
        )
    }

    fn create_causal_edge(
        &self,
        source_id: &str,
        target_id: &str,
        strength: f32,
        temporal_distance: i64,
    ) -> PendingEntanglementEdge {
        self.surreal_manager
            .create_causal_edge(source_id, target_id, strength, temporal_distance)
    }

    fn get_causal_predecessors(&self, memory_id: &str) -> MemoryStream {
        self.surreal_manager.get_causal_predecessors(memory_id)
    }

    fn get_causal_successors(&self, memory_id: &str) -> MemoryStream {
        self.surreal_manager.get_causal_successors(memory_id)
    }

    fn trace_causal_chain_forward(&self, start_memory_id: &str, max_depth: usize) -> MemoryStream {
        self.surreal_manager
            .trace_causal_chain_forward(start_memory_id, max_depth)
    }

    fn trace_causal_chain_backward(&self, start_memory_id: &str, max_depth: usize) -> MemoryStream {
        self.surreal_manager
            .trace_causal_chain_backward(start_memory_id, max_depth)
    }
}

// Optional: Add Drop implementation for automatic cleanup
impl Drop for MemoryCoordinator {
    fn drop(&mut self) {
        // Implement graceful shutdown of workers
        self.shutdown_workers();
    }
}
