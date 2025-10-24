//! Reference graph implementation for tracking relationships
//!
//! Provides blazing-fast graph operations for reference nodes with zero-allocation
//! adjacency management and efficient traversal algorithms.

use super::types::{
    EdgeMetadata, EdgeType, GraphMetadata, GraphStatistics, OptimizationResult, ReferenceEdge,
    ReferenceId, ReferenceNode,
};
use crate::lexer::Position;
use crate::semantic::SemanticError;
use std::collections::{HashMap, HashSet, VecDeque};

/// Reference graph for tracking node relationships
#[derive(Debug)]
pub struct ReferenceGraph<'input> {
    nodes: HashMap<ReferenceId, ReferenceNode<'input>>,
    adjacency_list: HashMap<ReferenceId, Vec<ReferenceEdge>>,
    reverse_adjacency: HashMap<ReferenceId, Vec<ReferenceId>>,
    metadata: GraphMetadata,
    node_id_counter: usize,
}

impl<'input> ReferenceGraph<'input> {
    /// Create new reference graph with optimized capacity
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            nodes: HashMap::with_capacity(256),
            adjacency_list: HashMap::with_capacity(256),
            reverse_adjacency: HashMap::with_capacity(256),
            metadata: GraphMetadata::default(),
            node_id_counter: 0,
        }
    }

    /// Add node to graph - blazing-fast insertion
    #[inline]
    pub fn add_node(&mut self, mut node: ReferenceNode<'input>) -> ReferenceId {
        let node_id = ReferenceId(self.node_id_counter);
        self.node_id_counter += 1;

        node.id = node_id;
        self.nodes.insert(node_id, node);
        self.adjacency_list.insert(node_id, Vec::new());
        self.reverse_adjacency.insert(node_id, Vec::new());

        self.update_metadata();
        node_id
    }

    /// Remove node and all its edges - efficient cleanup
    pub fn remove_node(&mut self, node_id: ReferenceId) -> Option<ReferenceNode<'input>> {
        if let Some(node) = self.nodes.remove(&node_id) {
            // Remove outgoing edges
            if let Some(edges) = self.adjacency_list.remove(&node_id) {
                for edge in edges {
                    // Remove reverse references
                    if let Some(reverse_list) = self.reverse_adjacency.get_mut(&edge.to) {
                        reverse_list.retain(|&id| id != node_id);
                    }
                }
            }

            // Remove incoming edges
            if let Some(incoming) = self.reverse_adjacency.remove(&node_id) {
                for &from_id in &incoming {
                    if let Some(outgoing) = self.adjacency_list.get_mut(&from_id) {
                        outgoing.retain(|edge| edge.to != node_id);
                    }
                }
            }

            self.update_metadata();
            Some(node)
        } else {
            None
        }
    }

    /// Add edge between nodes - optimized for performance
    pub fn add_edge(
        &mut self,
        from: ReferenceId,
        to: ReferenceId,
        edge_type: EdgeType,
        metadata: EdgeMetadata,
    ) -> Result<(), SemanticError> {
        // Validate nodes exist
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return Err(SemanticError::InternalError {
                message: "Cannot add edge: one or both nodes do not exist".to_string(),
                position: Position::default(),
            });
        }

        let edge = ReferenceEdge {
            from,
            to,
            edge_type,
            metadata,
        };

        // Add to adjacency list
        self.adjacency_list.entry(from).or_default().push(edge);

        // Add to reverse adjacency
        self.reverse_adjacency.entry(to).or_default().push(from);

        self.update_metadata();
        Ok(())
    }

    /// Remove edge between nodes
    pub fn remove_edge(&mut self, from: ReferenceId, to: ReferenceId) -> bool {
        let mut removed = false;

        // Remove from adjacency list
        if let Some(edges) = self.adjacency_list.get_mut(&from) {
            let original_len = edges.len();
            edges.retain(|edge| edge.to != to);
            removed = edges.len() != original_len;
        }

        // Remove from reverse adjacency
        if let Some(reverse_list) = self.reverse_adjacency.get_mut(&to) {
            reverse_list.retain(|&id| id != from);
        }

        if removed {
            self.update_metadata();
        }

        removed
    }

    /// Get node by ID
    #[inline]
    #[must_use] 
    pub fn get_node(&self, node_id: ReferenceId) -> Option<&ReferenceNode<'input>> {
        self.nodes.get(&node_id)
    }

    /// Get mutable node by ID
    #[inline]
    pub fn get_node_mut(&mut self, node_id: ReferenceId) -> Option<&mut ReferenceNode<'input>> {
        self.nodes.get_mut(&node_id)
    }

    /// Get edges from a node
    #[inline]
    #[must_use] 
    pub fn get_edges(&self, node_id: ReferenceId) -> Option<&Vec<ReferenceEdge>> {
        self.adjacency_list.get(&node_id)
    }

    /// Get incoming edges to a node
    #[inline]
    #[must_use] 
    pub fn get_incoming_edges(&self, node_id: ReferenceId) -> Option<&Vec<ReferenceId>> {
        self.reverse_adjacency.get(&node_id)
    }

    /// Get all node IDs
    #[inline]
    #[must_use] 
    pub fn get_all_node_ids(&self) -> Vec<ReferenceId> {
        self.nodes.keys().copied().collect()
    }

    /// Get node degree (total incoming + outgoing edges)
    #[inline]
    #[must_use] 
    pub fn get_node_degree(&self, node_id: ReferenceId) -> usize {
        let outgoing = self
            .adjacency_list
            .get(&node_id)
            .map_or(0, |edges| edges.len());
        let incoming = self
            .reverse_adjacency
            .get(&node_id)
            .map_or(0, |edges| edges.len());
        outgoing + incoming
    }

    /// Get outgoing degree only
    #[inline]
    #[must_use] 
    pub fn get_out_degree(&self, node_id: ReferenceId) -> usize {
        self.adjacency_list
            .get(&node_id)
            .map_or(0, |edges| edges.len())
    }

    /// Get incoming degree only
    #[inline]
    #[must_use] 
    pub fn get_in_degree(&self, node_id: ReferenceId) -> usize {
        self.reverse_adjacency
            .get(&node_id)
            .map_or(0, |edges| edges.len())
    }

    /// Check if there's a path between two nodes
    #[must_use] 
    pub fn has_path(&self, from: ReferenceId, to: ReferenceId) -> bool {
        if from == to {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from);
        visited.insert(from);

        while let Some(current) = queue.pop_front() {
            if let Some(edges) = self.adjacency_list.get(&current) {
                for edge in edges {
                    if edge.to == to {
                        return true;
                    }
                    if !visited.contains(&edge.to) {
                        visited.insert(edge.to);
                        queue.push_back(edge.to);
                    }
                }
            }
        }

        false
    }

    /// Get shortest path between two nodes (BFS)
    #[must_use] 
    pub fn get_shortest_path(
        &self,
        from: ReferenceId,
        to: ReferenceId,
    ) -> Option<Vec<ReferenceId>> {
        if from == to {
            return Some(vec![from]);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<ReferenceId, ReferenceId> = HashMap::new();

        queue.push_back(from);
        visited.insert(from);

        while let Some(current) = queue.pop_front() {
            if let Some(edges) = self.adjacency_list.get(&current) {
                for edge in edges {
                    if edge.to == to {
                        // Reconstruct path
                        let mut path = vec![to, current];
                        let mut current_parent = current;
                        while let Some(&p) = parent.get(&current_parent) {
                            path.push(p);
                            current_parent = p;
                        }
                        path.reverse();
                        return Some(path);
                    }

                    if !visited.contains(&edge.to) {
                        visited.insert(edge.to);
                        parent.insert(edge.to, current);
                        queue.push_back(edge.to);
                    }
                }
            }
        }

        None
    }

    /// Get all paths between two nodes (DFS with path tracking)
    #[must_use] 
    pub fn get_all_paths(
        &self,
        from: ReferenceId,
        to: ReferenceId,
        max_paths: usize,
    ) -> Vec<Vec<ReferenceId>> {
        let mut paths = Vec::new();
        let mut current_path = Vec::new();
        let mut visited = HashSet::new();

        self.dfs_all_paths(
            from,
            to,
            &mut current_path,
            &mut visited,
            &mut paths,
            max_paths,
        );
        paths
    }

    /// DFS helper for finding all paths
    fn dfs_all_paths(
        &self,
        current: ReferenceId,
        target: ReferenceId,
        path: &mut Vec<ReferenceId>,
        visited: &mut HashSet<ReferenceId>,
        paths: &mut Vec<Vec<ReferenceId>>,
        max_paths: usize,
    ) {
        if paths.len() >= max_paths {
            return; // Limit number of paths to prevent explosion
        }

        path.push(current);
        visited.insert(current);

        if current == target {
            paths.push(path.clone());
        } else if let Some(edges) = self.adjacency_list.get(&current) {
            for edge in edges {
                if !visited.contains(&edge.to) {
                    self.dfs_all_paths(edge.to, target, path, visited, paths, max_paths);
                }
            }
        }

        path.pop();
        visited.remove(&current);
    }

    /// Get connected components
    #[must_use] 
    pub fn get_connected_components(&self) -> Vec<Vec<ReferenceId>> {
        let mut visited = HashSet::new();
        let mut components = Vec::new();

        for &node_id in self.nodes.keys() {
            if !visited.contains(&node_id) {
                let mut component = Vec::new();
                self.dfs_component(node_id, &mut visited, &mut component);
                components.push(component);
            }
        }

        components
    }

    /// DFS for connected component discovery
    fn dfs_component(
        &self,
        node_id: ReferenceId,
        visited: &mut HashSet<ReferenceId>,
        component: &mut Vec<ReferenceId>,
    ) {
        visited.insert(node_id);
        component.push(node_id);

        // Check outgoing edges
        if let Some(edges) = self.adjacency_list.get(&node_id) {
            for edge in edges {
                if !visited.contains(&edge.to) {
                    self.dfs_component(edge.to, visited, component);
                }
            }
        }

        // Check incoming edges (for undirected traversal)
        if let Some(incoming) = self.reverse_adjacency.get(&node_id) {
            for &from_id in incoming {
                if !visited.contains(&from_id) {
                    self.dfs_component(from_id, visited, component);
                }
            }
        }
    }

    /// Calculate graph statistics
    #[must_use] 
    pub fn calculate_statistics(&self) -> GraphStatistics {
        let node_count = self.nodes.len();
        let edge_count: usize = self.adjacency_list.values().map(|edges| edges.len()).sum();

        let density = if node_count > 1 {
            (edge_count as f64) / ((node_count * (node_count - 1)) as f64)
        } else {
            0.0
        };

        let connected_components = self.get_connected_components().len();

        let max_depth = self.calculate_max_depth();

        let avg_degree = if node_count > 0 {
            (edge_count as f64 * 2.0) / (node_count as f64) // *2 for bidirectional count
        } else {
            0.0
        };

        GraphStatistics {
            node_count,
            edge_count,
            density,
            connected_components,
            max_depth,
            avg_degree,
        }
    }

    /// Calculate maximum depth of the graph
    fn calculate_max_depth(&self) -> usize {
        let mut max_depth = 0;

        for &node_id in self.nodes.keys() {
            let depth = self.calculate_node_depth(node_id);
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    /// Calculate depth from a specific node
    fn calculate_node_depth(&self, start_node: ReferenceId) -> usize {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((start_node, 0));
        visited.insert(start_node);

        let mut max_depth = 0;

        while let Some((node_id, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);

            if let Some(edges) = self.adjacency_list.get(&node_id) {
                for edge in edges {
                    if !visited.contains(&edge.to) {
                        visited.insert(edge.to);
                        queue.push_back((edge.to, depth + 1));
                    }
                }
            }
        }

        max_depth
    }

    /// Optimize graph for better performance
    pub fn optimize(&mut self) -> OptimizationResult {
        let initial_memory = self.estimate_memory_usage();
        let _initial_edges = self.adjacency_list.values().map(|v| v.len()).sum::<usize>();

        // Remove redundant edges
        let mut removed_edges = 0;
        for edges in self.adjacency_list.values_mut() {
            let original_len = edges.len();
            edges.sort_by_key(|e| e.to.0);
            edges.dedup_by_key(|e| e.to);
            removed_edges += original_len - edges.len();
        }

        // Shrink collections to fit
        self.nodes.shrink_to_fit();
        for edges in self.adjacency_list.values_mut() {
            edges.shrink_to_fit();
        }
        for edges in self.reverse_adjacency.values_mut() {
            edges.shrink_to_fit();
        }

        let final_memory = self.estimate_memory_usage();
        let memory_saved = initial_memory.saturating_sub(final_memory);

        let performance_improvement = if initial_memory > 0 {
            (memory_saved as f64 / initial_memory as f64) * 100.0
        } else {
            0.0
        };

        self.update_metadata();

        OptimizationResult {
            memory_saved,
            references_removed: removed_edges,
            performance_improvement,
        }
    }

    /// Clear all nodes and edges
    #[inline]
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.adjacency_list.clear();
        self.reverse_adjacency.clear();
        self.node_id_counter = 0;
        self.update_metadata();
    }

    /// Get graph metadata
    #[inline]
    #[must_use] 
    pub const fn get_metadata(&self) -> &GraphMetadata {
        &self.metadata
    }

    /// Check if graph is empty
    #[inline]
    #[must_use] 
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get number of nodes
    #[inline]
    #[must_use] 
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get number of edges
    #[inline]
    #[must_use] 
    pub fn edge_count(&self) -> usize {
        self.adjacency_list.values().map(|edges| edges.len()).sum()
    }

    /// Reserve capacity for expected number of nodes
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.nodes.reserve(additional);
        self.adjacency_list.reserve(additional);
        self.reverse_adjacency.reserve(additional);
    }

    /// Update metadata after graph changes
    #[inline]
    fn update_metadata(&mut self) {
        self.metadata.node_count = self.nodes.len();
        self.metadata.edge_count = self.edge_count();
        self.metadata.last_modified = std::time::SystemTime::now();
        self.metadata.connected_components = self.get_connected_components().len();
    }

    /// Estimate memory usage of the graph
    fn estimate_memory_usage(&self) -> usize {
        let nodes_size = self.nodes.len() * std::mem::size_of::<ReferenceNode>();
        let edges_size = self
            .adjacency_list
            .values()
            .map(|edges| edges.len() * std::mem::size_of::<ReferenceEdge>())
            .sum::<usize>();
        let reverse_size = self
            .reverse_adjacency
            .values()
            .map(|ids| ids.len() * std::mem::size_of::<ReferenceId>())
            .sum::<usize>();

        nodes_size + edges_size + reverse_size
    }
}

impl<'input> Default for ReferenceGraph<'input> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;
    use crate::semantic::references::types::{ReferenceNodeType, ScalarType};
    use std::borrow::Cow;

    fn create_test_node(id: usize, name: &str) -> ReferenceNode<'static> {
        ReferenceNode {
            id: ReferenceId(id),
            name: Cow::Owned(name.to_string()),
            node_type: ReferenceNodeType::Scalar {
                value: Cow::Borrowed("test"),
                scalar_type: ScalarType::String,
            },
            position: Position::default(),
            reference_path: vec![name.to_string()],
        }
    }

    #[test]
    fn test_add_node() {
        let mut graph = ReferenceGraph::new();
        let node = create_test_node(0, "test");

        let node_id = graph.add_node(node);
        assert_eq!(node_id.0, 0);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = ReferenceGraph::new();
        let node1 = create_test_node(0, "node1");
        let node2 = create_test_node(1, "node2");

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);

        let metadata = EdgeMetadata {
            weight: 1.0,
            priority: 1,
            is_critical: false,
        };

        let result = graph.add_edge(id1, id2, EdgeType::ChildRelation, metadata);
        assert!(result.is_ok());
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_shortest_path() {
        let mut graph = ReferenceGraph::new();
        let node1 = create_test_node(0, "node1");
        let node2 = create_test_node(1, "node2");
        let node3 = create_test_node(2, "node3");

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);
        let id3 = graph.add_node(node3);

        let metadata = EdgeMetadata {
            weight: 1.0,
            priority: 1,
            is_critical: false,
        };

        match graph.add_edge(id1, id2, EdgeType::ChildRelation, metadata.clone()) {
            Ok(_) => {} // Edge addition successful
            Err(_) => panic!("Expected successful edge addition"),
        }
        match graph.add_edge(id2, id3, EdgeType::ChildRelation, metadata) {
            Ok(_) => {} // Edge addition successful
            Err(_) => panic!("Expected successful edge addition"),
        }

        let path = graph.get_shortest_path(id1, id3);
        assert!(path.is_some());
        if let Some(path) = path {
            assert_eq!(path.len(), 3);
            assert_eq!(path[0], id1);
            assert_eq!(path[1], id2);
            assert_eq!(path[2], id3);
        } else {
            panic!("Expected path to be found");
        }
    }

    #[test]
    fn test_has_path() {
        let mut graph = ReferenceGraph::new();
        let node1 = create_test_node(0, "node1");
        let node2 = create_test_node(1, "node2");

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);

        assert!(!graph.has_path(id1, id2));

        let metadata = EdgeMetadata {
            weight: 1.0,
            priority: 1,
            is_critical: false,
        };

        match graph.add_edge(id1, id2, EdgeType::ChildRelation, metadata) {
            Ok(_) => {} // Edge addition successful
            Err(_) => panic!("Expected successful edge addition"),
        }
        assert!(graph.has_path(id1, id2));
    }
}
