//! Memory repository for managing in-memory cache and indexing

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::memory::filter::MemoryFilter;
use crate::memory::primitives::{MemoryNode, MemoryRelationship, MemoryTypeEnum};

/// In-memory repository for fast memory access and indexing
#[derive(Debug)]
pub struct MemoryRepository {
    /// Primary memory storage by ID
    memories: HashMap<String, Arc<MemoryNode>>,

    /// Index by memory type
    type_index: HashMap<MemoryTypeEnum, HashSet<String>>,

    /// Index by user ID
    user_index: HashMap<String, HashSet<String>>,

    /// Index by agent ID
    agent_index: HashMap<String, HashSet<String>>,

    /// Index by tags
    tag_index: HashMap<String, HashSet<String>>,

    /// Time-based index (sorted by creation time)
    time_index: BTreeMap<DateTime<Utc>, HashSet<String>>,

    /// Relationships storage
    relationships: HashMap<String, Vec<MemoryRelationship>>,
}

impl MemoryRepository {
    /// Create a new memory repository
    pub fn new() -> Self {
        Self {
            memories: HashMap::new(),
            type_index: HashMap::new(),
            user_index: HashMap::new(),
            agent_index: HashMap::new(),
            tag_index: HashMap::new(),
            time_index: BTreeMap::new(),
            relationships: HashMap::new(),
        }
    }

    /// Create and add a memory to the repository
    pub fn create(
        &mut self,
        id: &str,
        memory: &MemoryNode,
    ) -> crate::memory::utils::Result<MemoryNode> {
        // Create a new memory with the provided ID
        let mut new_memory = memory.clone();
        new_memory.id = id.to_string();

        // Add to repository
        self.add(new_memory.clone());

        Ok(new_memory)
    }

    /// Add a memory to the repository
    pub fn add(&mut self, memory: MemoryNode) {
        let memory_arc = Arc::new(memory);
        let memory_ref = &memory_arc;

        // Add to primary storage
        self.memories
            .insert(memory_ref.id.clone(), memory_arc.clone());

        // Add to indexes
        self.type_index
            .entry(memory_ref.memory_type)
            .or_default()
            .insert(memory_ref.id.clone());

        if let Some(user_id) = &memory_ref.metadata.user_id {
            self.user_index
                .entry(user_id.clone())
                .or_default()
                .insert(memory_ref.id.clone());
        }

        if let Some(agent_id) = &memory_ref.metadata.agent_id {
            self.agent_index
                .entry(agent_id.clone())
                .or_default()
                .insert(memory_ref.id.clone());
        }

        for tag in &memory_ref.metadata.tags {
            self.tag_index
                .entry(tag.clone())
                .or_default()
                .insert(memory_ref.id.clone());
        }

        self.time_index
            .entry(memory_ref.created_at)
            .or_default()
            .insert(memory_ref.id.clone());
    }

    /// Get a memory by its ID
    pub fn get(&self, id: &str) -> Option<Arc<MemoryNode>> {
        let result = self.memories.get(id).cloned();

        // Record cache performance metrics
        if result.is_some() {
            crate::domain::memory::ops::record_cache_hit();
        } else {
            crate::domain::memory::ops::record_cache_miss();
        }

        result
    }

    /// Delete a memory by its ID
    pub fn delete(&mut self, id: &str) -> Option<Arc<MemoryNode>> {
        if let Some(memory) = self.memories.remove(id) {
            self.remove_from_indexes(&memory);
            Some(memory)
        } else {
            None
        }
    }

    /// Update a memory
    pub fn update(&mut self, memory: MemoryNode) -> Option<Arc<MemoryNode>> {
        if let Some(old_memory) = self.memories.get(&memory.id).cloned() {
            self.remove_from_indexes(&old_memory);
            self.add(memory.clone());
            self.memories.get(&memory.id).cloned()
        } else {
            None
        }
    }

    /// Filter memories based on criteria
    pub fn filter(&self, filter: &MemoryFilter) -> Vec<Arc<MemoryNode>> {
        let mut results: Vec<Arc<MemoryNode>> = Vec::new();

        // Start with all memories if no specific index is used
        let initial_set: HashSet<String> = if let Some(user_id) = &filter.user_id {
            self.user_index.get(user_id).cloned().unwrap_or_default()
        } else if let Some(agent_id) = &filter.agent_id {
            self.agent_index.get(agent_id).cloned().unwrap_or_default()
        } else {
            self.memories.keys().cloned().collect()
        };

        for id in initial_set {
            if let Some(memory) = self.memories.get(&id)
                && filter.matches(memory)
            {
                results.push(memory.clone());
            }
        }

        // Sort results if needed
        if let Some(sort_by) = &filter.sort_by {
            results.sort_by(|a, b| match sort_by.as_str() {
                "created_at" => a.created_at.cmp(&b.created_at),
                "updated_at" => a.updated_at.cmp(&b.updated_at),
                "importance" => a
                    .metadata
                    .importance
                    .partial_cmp(&b.metadata.importance)
                    .unwrap_or(std::cmp::Ordering::Equal), // Handle NaN cases gracefully
                _ => std::cmp::Ordering::Equal,
            });
        }

        if filter.sort_descending {
            results.reverse();
        }

        results
            .into_iter()
            .take(filter.limit.unwrap_or(usize::MAX))
            .collect()
    }

    /// Add a relationship between two memories
    pub fn add_relationship(&mut self, relationship: MemoryRelationship) {
        self.relationships
            .entry(relationship.source_id.clone())
            .or_default()
            .push(relationship);
    }

    /// Get relationships for a memory
    pub fn get_relationships(&self, memory_id: &str) -> Vec<MemoryRelationship> {
        self.relationships
            .get(memory_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get repository statistics
    pub fn get_stats(&self) -> RepositoryStats {
        RepositoryStats {
            total_memories: self.memories.len(),
            memories_by_type: self
                .type_index
                .iter()
                .map(|(t, ids)| (*t, ids.len()))
                .collect(),
            total_relationships: self.relationships.values().map(|v| v.len()).sum(),
            unique_users: self.user_index.len(),
            unique_agents: self.agent_index.len(),
            unique_tags: self.tag_index.len(),
        }
    }

    /// Remove a memory from all indexes
    fn remove_from_indexes(&mut self, memory: &MemoryNode) {
        // Remove from type index
        if let Some(type_ids) = self.type_index.get_mut(&memory.memory_type) {
            type_ids.remove(&memory.id);
        }

        // Remove from user index
        if let Some(user_id) = &memory.metadata.user_id
            && let Some(user_ids) = self.user_index.get_mut(user_id)
        {
            user_ids.remove(&memory.id);
        }

        // Remove from agent index
        if let Some(agent_id) = &memory.metadata.agent_id
            && let Some(agent_ids) = self.agent_index.get_mut(agent_id)
        {
            agent_ids.remove(&memory.id);
        }

        // Remove from tag index
        for tag in &memory.metadata.tags {
            if let Some(tag_ids) = self.tag_index.get_mut(tag) {
                tag_ids.remove(&memory.id);
            }
        }

        // Remove from time index
        if let Some(time_ids) = self.time_index.get_mut(&memory.created_at) {
            time_ids.remove(&memory.id);
        }
    }
}

impl Default for MemoryRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Repository statistics
#[derive(Debug, Clone)]
pub struct RepositoryStats {
    /// Total number of memories
    pub total_memories: usize,

    /// Number of memories by type
    pub memories_by_type: HashMap<MemoryTypeEnum, usize>,

    /// Total number of relationships
    pub total_relationships: usize,

    /// Number of unique users
    pub unique_users: usize,

    /// Number of unique agents
    pub unique_agents: usize,

    /// Number of unique tags
    pub unique_tags: usize,
}
