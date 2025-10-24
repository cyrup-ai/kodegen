//! A graph of memory relationships

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::memory::primitives::RelationshipType;

/// A graph of memory relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipGraph {
    /// Adjacency list for outgoing edges
    edges: HashMap<String, Vec<(String, RelationshipType, f32)>>,
    /// Adjacency list for incoming edges
    reverse_edges: HashMap<String, Vec<(String, RelationshipType, f32)>>,
}

impl RelationshipGraph {
    /// Create a new relationship graph
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    /// Add a relationship to the graph
    pub fn add_relationship(
        &mut self,
        source: &str,
        target: &str,
        rel_type: RelationshipType,
        weight: f32,
    ) {
        self.edges.entry(source.to_string()).or_default().push((
            target.to_string(),
            rel_type.clone(),
            weight,
        ));

        self.reverse_edges
            .entry(target.to_string())
            .or_default()
            .push((source.to_string(), rel_type.clone(), weight));

        if rel_type.is_bidirectional() {
            self.edges.entry(target.to_string()).or_default().push((
                source.to_string(),
                rel_type.clone(),
                weight,
            ));
            self.reverse_edges
                .entry(source.to_string())
                .or_default()
                .push((target.to_string(), rel_type, weight));
        }
    }

    /// Get outgoing relationships for a memory
    pub fn get_outgoing(&self, memory_id: &str) -> Vec<(String, RelationshipType, f32)> {
        self.edges.get(memory_id).cloned().unwrap_or_default()
    }

    /// Get incoming relationships for a memory
    pub fn get_incoming(&self, memory_id: &str) -> Vec<(String, RelationshipType, f32)> {
        self.reverse_edges
            .get(memory_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Find path between two memories
    pub fn find_path(&self, start: &str, end: &str, max_depth: usize) -> Option<Vec<String>> {
        // Simple BFS implementation
        use std::collections::{HashSet, VecDeque};

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent_map: HashMap<String, String> = HashMap::new();

        queue.push_back(start.to_string());
        visited.insert(start.to_string());

        while let Some(current) = queue.pop_front() {
            if current == end {
                // Reconstruct path
                let mut path = vec![end.to_string()];
                let mut node = end.to_string();

                while let Some(parent) = parent_map.get(&node) {
                    path.push(parent.clone());
                    node = parent.clone();
                }

                path.reverse();
                return Some(path);
            }

            if visited.len() >= max_depth {
                continue;
            }

            if let Some(neighbors) = self.edges.get(&current) {
                for (neighbor, _, _) in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        parent_map.insert(neighbor.clone(), current.clone());
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        None
    }
}

impl Default for RelationshipGraph {
    fn default() -> Self {
        Self::new()
    }
}
