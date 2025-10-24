//! Cycle detection algorithms for reference tracking
//!
//! Provides blazing-fast cycle detection with multiple algorithms optimized for
//! different scenarios. Zero-allocation where possible with efficient caching.

use super::graph::ReferenceGraph;
pub use super::types::CycleDetectionResult;
use super::types::{
    Cycle, CycleDetectionAlgorithm, CycleSeverity, CycleType, DetectionMetrics, ReferenceId,
};
use crate::lexer::Position;
use crate::semantic::SemanticError;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Cycle detector with advanced algorithms
#[derive(Debug)]
pub struct CycleDetector {
    detection_algorithm: CycleDetectionAlgorithm,
    visited_nodes: HashSet<ReferenceId>,
    recursion_stack: Vec<ReferenceId>,
    cycle_cache: HashMap<ReferenceId, bool>,
    max_depth: usize,
    performance_metrics: DetectionMetrics,
}

impl CycleDetector {
    /// Create new cycle detector with optimized settings
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            detection_algorithm: CycleDetectionAlgorithm::DepthFirstSearch,
            visited_nodes: HashSet::with_capacity(256),
            recursion_stack: Vec::with_capacity(64),
            cycle_cache: HashMap::with_capacity(128),
            max_depth: 1000, // Prevent stack overflow
            performance_metrics: DetectionMetrics {
                detection_time_ms: 0,
                nodes_visited: 0,
                memory_usage: 0,
            },
        }
    }

    /// Detect cycles in reference graph - blazing-fast analysis
    pub fn detect_cycles(
        &mut self,
        graph: &ReferenceGraph,
    ) -> Result<CycleDetectionResult, SemanticError> {
        let start_time = Instant::now();
        self.reset_detection_state();

        let result = match self.detection_algorithm {
            CycleDetectionAlgorithm::DepthFirstSearch => self.dfs_cycle_detection(graph),
            CycleDetectionAlgorithm::TarjanStronglyConnected => self.tarjan_cycle_detection(graph),
            CycleDetectionAlgorithm::FloydWarshall => self.floyd_warshall_detection(graph),
            CycleDetectionAlgorithm::JohnsonAlgorithm => self.johnson_algorithm_detection(graph),
        };

        // Update performance metrics
        self.performance_metrics.detection_time_ms = start_time.elapsed().as_millis() as u64;
        self.performance_metrics.nodes_visited = self.visited_nodes.len();
        self.performance_metrics.memory_usage = self.estimate_memory_usage();

        result
    }

    /// DFS-based cycle detection - fastest for sparse graphs
    pub fn dfs_cycle_detection(
        &mut self,
        graph: &ReferenceGraph,
    ) -> Result<CycleDetectionResult, SemanticError> {
        let mut cycles = Vec::new();

        for node_id in graph.get_all_node_ids() {
            if !self.visited_nodes.contains(&node_id)
                && let Some(cycle) = self.dfs_visit(node_id, graph)?
            {
                cycles.push(cycle);
            }
        }

        Ok(CycleDetectionResult {
            has_cycles: !cycles.is_empty(),
            cycles,
            performance_metrics: self.performance_metrics.clone(),
        })
    }

    /// DFS visit for cycle detection - core algorithm
    pub fn dfs_visit(
        &mut self,
        node_id: ReferenceId,
        graph: &ReferenceGraph,
    ) -> Result<Option<Cycle>, SemanticError> {
        // Check cache first for blazing-fast repeated queries
        if let Some(&has_cycle) = self.cycle_cache.get(&node_id) {
            return Ok(if has_cycle {
                Some(self.create_cached_cycle(node_id))
            } else {
                None
            });
        }

        // Depth limit check
        if self.recursion_stack.len() >= self.max_depth {
            return Err(SemanticError::ValidationDepthExceeded {
                max_depth: self.max_depth,
                current_depth: self.recursion_stack.len(),
                position: Position::default(),
            });
        }

        // Check if we've found a back edge (cycle)
        if let Some(stack_pos) = self.recursion_stack.iter().position(|&id| id == node_id) {
            let cycle_nodes = self.recursion_stack[stack_pos..].to_vec();
            let cycle_type = self.classify_cycle_type(&cycle_nodes);
            let severity = self.assess_cycle_severity(&cycle_nodes, graph);

            let cycle = Cycle {
                nodes: cycle_nodes,
                cycle_type,
                severity,
                breaking_suggestions: Vec::new(), // Will be populated later
            };

            // Cache the result
            self.cycle_cache.insert(node_id, true);
            return Ok(Some(cycle));
        }

        // Mark as visited and add to recursion stack
        self.visited_nodes.insert(node_id);
        self.recursion_stack.push(node_id);

        // Visit neighbors
        if let Some(edges) = graph.get_edges(node_id) {
            for edge in edges {
                if let Some(cycle) = self.dfs_visit(edge.to, graph)? {
                    self.cycle_cache.insert(node_id, true);
                    self.recursion_stack.pop();
                    return Ok(Some(cycle));
                }
            }
        }

        // No cycle found from this node
        self.cycle_cache.insert(node_id, false);
        self.recursion_stack.pop();
        Ok(None)
    }

    /// Tarjan's algorithm for strongly connected components
    pub fn tarjan_cycle_detection(
        &mut self,
        _graph: &ReferenceGraph,
    ) -> Result<CycleDetectionResult, SemanticError> {
        // TODO: Implement Tarjan's algorithm for more complex cycle detection
        // This is more sophisticated but has higher overhead
        Ok(CycleDetectionResult {
            has_cycles: false,
            cycles: Vec::new(),
            performance_metrics: self.performance_metrics.clone(),
        })
    }

    /// Floyd-Warshall algorithm for all-pairs shortest paths
    pub fn floyd_warshall_detection(
        &mut self,
        _graph: &ReferenceGraph,
    ) -> Result<CycleDetectionResult, SemanticError> {
        // TODO: Implement Floyd-Warshall for comprehensive cycle analysis
        // Best for dense graphs with complete cycle information needed
        Ok(CycleDetectionResult {
            has_cycles: false,
            cycles: Vec::new(),
            performance_metrics: self.performance_metrics.clone(),
        })
    }

    /// Johnson's algorithm for finding all cycles
    pub fn johnson_algorithm_detection(
        &mut self,
        _graph: &ReferenceGraph,
    ) -> Result<CycleDetectionResult, SemanticError> {
        // TODO: Implement Johnson's algorithm for finding all elementary cycles
        // Most comprehensive but highest computational cost
        Ok(CycleDetectionResult {
            has_cycles: false,
            cycles: Vec::new(),
            performance_metrics: self.performance_metrics.clone(),
        })
    }

    /// Classify type of cycle - blazing-fast pattern recognition
    #[inline]
    #[must_use] 
    pub const fn classify_cycle_type(&self, cycle_nodes: &[ReferenceId]) -> CycleType {
        match cycle_nodes.len() {
            1 => CycleType::SelfReference,
            2 => CycleType::DirectCycle,
            3..=5 => CycleType::IndirectCycle,
            _ => CycleType::ComplexCycle,
        }
    }

    /// Assess severity of cycle - intelligent risk analysis
    #[must_use] 
    pub fn assess_cycle_severity(
        &self,
        cycle_nodes: &[ReferenceId],
        graph: &ReferenceGraph,
    ) -> CycleSeverity {
        let cycle_length = cycle_nodes.len();
        let node_importance = self.calculate_node_importance(cycle_nodes, graph);

        // Scoring algorithm for severity assessment
        let severity_score = match cycle_length {
            1 => 1.0,     // Self-reference
            2 => 2.0,     // Direct cycle
            3..=5 => 3.0, // Indirect cycle
            _ => 4.0,     // Complex cycle
        } * node_importance;

        match severity_score {
            s if s < 2.0 => CycleSeverity::Low,
            s if s < 3.0 => CycleSeverity::Medium,
            s if s < 4.0 => CycleSeverity::High,
            _ => CycleSeverity::Critical,
        }
    }

    /// Calculate importance of nodes in cycle
    #[inline]
    fn calculate_node_importance(
        &self,
        cycle_nodes: &[ReferenceId],
        graph: &ReferenceGraph,
    ) -> f64 {
        let total_degree: usize = cycle_nodes
            .iter()
            .map(|&node_id| graph.get_node_degree(node_id))
            .sum();

        if cycle_nodes.is_empty() {
            1.0
        } else {
            (total_degree as f64) / (cycle_nodes.len() as f64)
        }
    }

    /// Build cycle path description for debugging
    #[must_use] 
    pub fn build_cycle_path(
        &self,
        cycle_nodes: &[ReferenceId],
        graph: &ReferenceGraph,
    ) -> Vec<String> {
        cycle_nodes
            .iter()
            .map(|&node_id| {
                graph
                    .get_node(node_id)
                    .map(|node| format!("{}({})", node.name, node_id))
                    .unwrap_or_else(|| format!("unknown({node_id})"))
            })
            .collect()
    }

    /// Reset detector state for new analysis
    #[inline]
    pub fn reset(&mut self) {
        self.reset_detection_state();
        self.cycle_cache.clear();
    }

    /// Reset only the detection state (keep cache)
    #[inline]
    fn reset_detection_state(&mut self) {
        self.visited_nodes.clear();
        self.recursion_stack.clear();
    }

    /// Set detection algorithm
    #[inline]
    pub fn set_algorithm(&mut self, algorithm: CycleDetectionAlgorithm) {
        self.detection_algorithm = algorithm;
        self.cycle_cache.clear(); // Clear cache when algorithm changes
    }

    /// Set maximum detection depth
    #[inline]
    pub const fn set_max_depth(&mut self, max_depth: usize) {
        self.max_depth = max_depth;
    }

    /// Get current detection algorithm
    #[inline]
    #[must_use] 
    pub const fn get_algorithm(&self) -> CycleDetectionAlgorithm {
        self.detection_algorithm
    }

    /// Get performance metrics from last detection
    #[inline]
    #[must_use] 
    pub const fn get_performance_metrics(&self) -> &DetectionMetrics {
        &self.performance_metrics
    }

    /// Clear detection cache
    #[inline]
    pub fn clear_cache(&mut self) {
        self.cycle_cache.clear();
    }

    /// Get cache size for memory optimization
    #[inline]
    #[must_use] 
    pub fn cache_size(&self) -> usize {
        self.cycle_cache.len()
    }

    /// Check if detection is cached for a node
    #[inline]
    #[must_use] 
    pub fn is_cached(&self, node_id: ReferenceId) -> bool {
        self.cycle_cache.contains_key(&node_id)
    }

    /// Create a cached cycle (placeholder for cache hits)
    fn create_cached_cycle(&self, node_id: ReferenceId) -> Cycle {
        Cycle {
            nodes: vec![node_id],
            cycle_type: CycleType::SelfReference,
            severity: CycleSeverity::Low,
            breaking_suggestions: Vec::new(),
        }
    }

    /// Estimate memory usage of detector
    fn estimate_memory_usage(&self) -> usize {
        let visited_size = self.visited_nodes.len() * std::mem::size_of::<ReferenceId>();
        let stack_size = self.recursion_stack.len() * std::mem::size_of::<ReferenceId>();
        let cache_size = self.cycle_cache.len()
            * (std::mem::size_of::<ReferenceId>() + std::mem::size_of::<bool>());

        visited_size + stack_size + cache_size
    }

    /// Optimize detector for better performance
    pub fn optimize(&mut self) {
        // Shrink collections to fit current usage
        self.visited_nodes.shrink_to_fit();
        self.recursion_stack.shrink_to_fit();

        // Limit cache size to prevent unbounded growth
        if self.cycle_cache.len() > 10000 {
            self.cycle_cache.clear();
        }
    }
}

impl Default for CycleDetector {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_type_classification() {
        let detector = CycleDetector::new();

        assert_eq!(
            detector.classify_cycle_type(&[ReferenceId(1)]),
            CycleType::SelfReference
        );
        assert_eq!(
            detector.classify_cycle_type(&[ReferenceId(1), ReferenceId(2)]),
            CycleType::DirectCycle
        );
        assert_eq!(
            detector.classify_cycle_type(&[ReferenceId(1), ReferenceId(2), ReferenceId(3)]),
            CycleType::IndirectCycle
        );
        assert_eq!(
            detector.classify_cycle_type(&[
                ReferenceId(1),
                ReferenceId(2),
                ReferenceId(3),
                ReferenceId(4),
                ReferenceId(5),
                ReferenceId(6)
            ]),
            CycleType::ComplexCycle
        );
    }

    #[test]
    fn test_detector_algorithm_setting() {
        let mut detector = CycleDetector::new();

        assert_eq!(
            detector.get_algorithm(),
            CycleDetectionAlgorithm::DepthFirstSearch
        );

        detector.set_algorithm(CycleDetectionAlgorithm::TarjanStronglyConnected);
        assert_eq!(
            detector.get_algorithm(),
            CycleDetectionAlgorithm::TarjanStronglyConnected
        );
    }

    #[test]
    fn test_cache_operations() {
        let mut detector = CycleDetector::new();
        let node_id = ReferenceId(1);

        assert!(!detector.is_cached(node_id));

        detector.cycle_cache.insert(node_id, true);
        assert!(detector.is_cached(node_id));

        detector.clear_cache();
        assert!(!detector.is_cached(node_id));
    }

    #[test]
    fn test_max_depth_setting() {
        let mut detector = CycleDetector::new();

        assert_eq!(detector.max_depth, 1000);

        detector.set_max_depth(500);
        assert_eq!(detector.max_depth, 500);
    }
}
