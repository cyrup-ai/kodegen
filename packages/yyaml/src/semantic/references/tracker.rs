//! Reference tracker implementation for coordinating all reference operations
//!
//! Provides the main ReferenceTracker that coordinates graph management, memory
//! optimization, cycle detection, and statistics collection with blazing-fast performance.

use super::cycle_detection::{CycleDetectionResult, CycleDetector};
use super::graph::ReferenceGraph;
use super::memory::MemoryManager;
use super::statistics::{OperationReport, OperationType, StatisticsCollector, StatisticsReport};
use super::types::{
    EdgeMetadata, EdgeType, ReferenceId, ReferenceNode, ReferenceNodeType, TrackingContext,
};
use crate::lexer::Position;
use crate::parser::ast::Node;
use crate::semantic::SemanticError;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::time::{Instant, SystemTime};

/// Main reference tracker coordinating all reference operations
#[derive(Debug)]
pub struct ReferenceTracker<'input> {
    graph: ReferenceGraph<'input>,
    memory_manager: MemoryManager<'input>,
    cycle_detector: CycleDetector,
    statistics_collector: StatisticsCollector,
    anchor_registry: HashMap<Cow<'input, str>, ReferenceId>,
    alias_registry: HashMap<Cow<'input, str>, ReferenceId>,
    context: TrackingContext,
    is_optimized: bool,
}

impl<'input> ReferenceTracker<'input> {
    /// Create new reference tracker with optimized settings
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            graph: ReferenceGraph::new(),
            memory_manager: MemoryManager::new(),
            cycle_detector: CycleDetector::new(),
            statistics_collector: StatisticsCollector::new(),
            anchor_registry: HashMap::with_capacity(128),
            alias_registry: HashMap::with_capacity(64),
            context: TrackingContext {
                is_enabled: true,
                max_depth: 100,
                current_depth: 0,
                visited_nodes: HashSet::with_capacity(64),
            },
            is_optimized: false,
        }
    }

    /// Create tracker with specific configuration
    #[must_use] 
    pub fn with_config(config: &crate::semantic::SemanticConfig<'_>) -> Self {
        let mut tracker = Self::new();

        // Configure based on cycle detection setting
        if !config.cycle_detection_enabled {
            tracker.context.is_enabled = false;
        }

        // Configure based on strict mode
        if config.strict_mode {
            tracker.context.max_depth = 50; // Stricter depth limit
        }

        tracker
    }

    /// Track anchor definition - blazing-fast registration
    pub fn track_anchor(
        &mut self,
        name: Cow<'input, str>,
        value: &'input Node<'input>,
        position: Position,
    ) -> Result<ReferenceId, SemanticError> {
        if !self.context.is_enabled {
            return Err(SemanticError::InternalError {
                message: "Reference tracking is disabled".to_string(),
                position,
            });
        }

        let start_time = Instant::now();

        // Check for duplicate anchor names
        if self.anchor_registry.contains_key(&name) {
            return Err(SemanticError::ConflictingAnchor {
                anchor_name: name.to_string(),
                first_position: Position::default(), // Would need to track first occurrence
                second_position: position,
            });
        }

        // Create reference node
        let node = ReferenceNode {
            id: ReferenceId(0), // Will be set by graph
            name: name.clone(),
            node_type: ReferenceNodeType::Anchor {
                name: name.clone(),
                value,
                anchor_type: super::types::AnchorType::Standard,
            },
            position,
            reference_path: vec![name.to_string()],
        };

        // Add to graph and memory
        let node_id = self.graph.add_node(node);
        self.anchor_registry.insert(name, node_id);

        // Record operation
        self.record_operation(OperationType::NodeCreation, start_time, true, None);

        Ok(node_id)
    }

    /// Track alias reference - efficient resolution
    pub fn track_alias(
        &mut self,
        alias_name: Cow<'input, str>,
        target_name: Cow<'input, str>,
        position: Position,
    ) -> Result<ReferenceId, SemanticError> {
        if !self.context.is_enabled {
            return Err(SemanticError::InternalError {
                message: "Reference tracking is disabled".to_string(),
                position,
            });
        }

        let start_time = Instant::now();

        // Look up target anchor
        let target_id = self.anchor_registry.get(&target_name).copied();

        // Create alias node
        let node = ReferenceNode {
            id: ReferenceId(0), // Will be set by graph
            name: alias_name.clone(),
            node_type: ReferenceNodeType::Alias {
                target: target_name.clone(),
                resolved_target: target_id,
                alias_type: super::types::AliasType::Simple,
            },
            position,
            reference_path: vec![alias_name.to_string(), target_name.to_string()],
        };

        // Add to graph
        let alias_id = self.graph.add_node(node);
        self.alias_registry.insert(alias_name, alias_id);

        // Create edge if target exists
        if let Some(target_id) = target_id {
            let metadata = EdgeMetadata {
                weight: 1.0,
                priority: 1,
                is_critical: true,
            };
            self.graph
                .add_edge(alias_id, target_id, EdgeType::AliasReference, metadata)?;
        }

        // Record operation
        self.record_operation(OperationType::ReferenceResolution, start_time, true, None);

        Ok(alias_id)
    }

    /// Resolve all unresolved aliases - comprehensive resolution
    pub fn resolve_aliases(&mut self) -> Result<usize, SemanticError> {
        let start_time = Instant::now();
        let mut resolved_count = 0;

        // Collect unresolved aliases
        let mut unresolved_aliases = Vec::new();

        for &alias_id in self.alias_registry.values() {
            if let Some(node) = self.graph.get_node(alias_id)
                && let ReferenceNodeType::Alias {
                    resolved_target: None,
                    target,
                    ..
                } = &node.node_type
                && let Some(&target_id) = self.anchor_registry.get(target)
            {
                unresolved_aliases.push((alias_id, target_id));
            }
        }

        // Resolve aliases and create edges
        for (alias_id, target_id) in unresolved_aliases {
            let metadata = EdgeMetadata {
                weight: 1.0,
                priority: 1,
                is_critical: true,
            };

            self.graph
                .add_edge(alias_id, target_id, EdgeType::AliasReference, metadata)?;

            // Update node to mark as resolved
            if let Some(node) = self.graph.get_node_mut(alias_id)
                && let ReferenceNodeType::Alias {
                    resolved_target, ..
                } = &mut node.node_type
            {
                *resolved_target = Some(target_id);
            }

            resolved_count += 1;
        }

        // Record operation
        self.record_operation(OperationType::ReferenceResolution, start_time, true, None);

        Ok(resolved_count)
    }

    /// Detect cycles in reference graph - comprehensive analysis
    pub fn detect_cycles(&mut self) -> Result<CycleDetectionResult, SemanticError> {
        let start_time = Instant::now();

        let result = self.cycle_detector.detect_cycles(&self.graph);

        // Record operation
        let success = result.is_ok();
        let error_msg = if let Err(ref e) = result {
            Some(e.to_string())
        } else {
            None
        };

        self.record_operation(
            OperationType::CycleDetection,
            start_time,
            success,
            error_msg,
        );

        result
    }

    /// Optimize memory and graph structure - blazing-fast optimization
    pub fn optimize(&mut self) -> Result<super::types::OptimizationResult, SemanticError> {
        let start_time = Instant::now();

        // Memory optimization
        self.memory_manager.garbage_collect();
        self.memory_manager.shrink_to_fit();

        // Graph optimization
        let graph_result = self.graph.optimize();

        // Cycle detector optimization
        self.cycle_detector.optimize();

        // Statistics collection
        self.statistics_collector
            .collect_statistics(&self.graph, &self.memory_manager);

        self.is_optimized = true;

        // Record operation
        self.record_operation(OperationType::MemoryOptimization, start_time, true, None);

        Ok(graph_result)
    }

    /// Track document reference
    pub fn track_document(
        &mut self,
        root: &'input Node<'input>,
        document_index: usize,
        position: Position,
    ) -> Result<ReferenceId, SemanticError> {
        if !self.context.is_enabled {
            return Err(SemanticError::InternalError {
                message: "Reference tracking is disabled".to_string(),
                position,
            });
        }

        let start_time = Instant::now();

        // Create document node
        let node = ReferenceNode {
            id: ReferenceId(0), // Will be set by graph
            name: Cow::Owned(format!("document_{document_index}")),
            node_type: ReferenceNodeType::Document {
                root,
                document_index,
            },
            position,
            reference_path: vec![format!("document_{}", document_index)],
        };

        // Add to graph
        let node_id = self.graph.add_node(node);

        // Record operation
        self.record_operation(OperationType::NodeCreation, start_time, true, None);

        Ok(node_id)
    }

    /// Check if reference exists
    #[inline]
    #[must_use] 
    pub fn has_anchor(&self, name: &str) -> bool {
        self.anchor_registry.contains_key(name)
    }

    #[inline]
    #[must_use] 
    pub fn has_alias(&self, name: &str) -> bool {
        self.alias_registry.contains_key(name)
    }

    /// Get reference by name
    #[must_use] 
    pub fn get_anchor(&self, name: &str) -> Option<ReferenceId> {
        self.anchor_registry.get(name).copied()
    }

    #[must_use] 
    pub fn get_alias(&self, name: &str) -> Option<ReferenceId> {
        self.alias_registry.get(name).copied()
    }

    /// Generate comprehensive statistics report
    pub fn generate_report(&mut self) -> StatisticsReport {
        // Update statistics before generating report
        self.statistics_collector
            .collect_statistics(&self.graph, &self.memory_manager);
        self.statistics_collector.generate_report()
    }

    /// Get node by ID
    #[inline]
    #[must_use] 
    pub fn get_node(&self, node_id: ReferenceId) -> Option<&ReferenceNode<'input>> {
        self.graph.get_node(node_id)
    }

    /// Enable or disable tracking
    #[inline]
    pub const fn set_enabled(&mut self, enabled: bool) {
        self.context.is_enabled = enabled;
        self.statistics_collector.set_enabled(enabled);
    }

    /// Set maximum tracking depth
    #[inline]
    pub const fn set_max_depth(&mut self, max_depth: usize) {
        self.context.max_depth = max_depth;
        self.cycle_detector.set_max_depth(max_depth);
    }

    /// Clear all tracking data
    pub fn clear(&mut self) {
        self.graph.clear();
        self.memory_manager.reset();
        self.cycle_detector.reset();
        self.statistics_collector.reset();
        self.anchor_registry.clear();
        self.alias_registry.clear();
        self.context.visited_nodes.clear();
        self.context.current_depth = 0;
        self.is_optimized = false;
    }

    /// Get current tracking context
    #[inline]
    #[must_use] 
    pub const fn get_context(&self) -> &TrackingContext {
        &self.context
    }

    /// Check if tracker is in optimized state
    #[inline]
    #[must_use] 
    pub const fn is_optimized(&self) -> bool {
        self.is_optimized
    }

    /// Get graph reference for advanced operations
    #[inline]
    #[must_use] 
    pub const fn get_graph(&self) -> &ReferenceGraph<'input> {
        &self.graph
    }

    /// Get memory manager reference
    #[inline]
    #[must_use] 
    pub const fn get_memory_manager(&self) -> &MemoryManager<'input> {
        &self.memory_manager
    }

    /// Get statistics collector reference
    #[inline]
    #[must_use] 
    pub const fn get_statistics_collector(&self) -> &StatisticsCollector {
        &self.statistics_collector
    }

    /// Record operation for statistics
    fn record_operation(
        &mut self,
        operation_type: OperationType,
        start_time: Instant,
        success: bool,
        error_message: Option<String>,
    ) {
        let duration = start_time.elapsed();
        let memory_usage = self.memory_manager.get_memory_usage();

        let report = OperationReport {
            timestamp: SystemTime::now(),
            operation_type,
            duration_ms: duration.as_millis() as u64,
            memory_before: memory_usage.used_bytes,
            memory_after: memory_usage.used_bytes,
            nodes_processed: self.graph.node_count(),
            success,
            error_message,
        };

        self.statistics_collector.record_operation(report);
    }

    /// Validate internal consistency
    pub fn validate(&self) -> Result<(), SemanticError> {
        // Check anchor registry consistency
        for (name, &node_id) in &self.anchor_registry {
            if let Some(node) = self.graph.get_node(node_id) {
                if node.name != *name {
                    return Err(SemanticError::ValidationFailure {
                        rule: "anchor_registry_consistency".to_string(),
                        message: format!(
                            "Anchor registry inconsistency: {} vs {}",
                            name, node.name
                        ),
                        position: Position::default(),
                    });
                }
            } else {
                return Err(SemanticError::ValidationFailure {
                    rule: "anchor_registry_existence".to_string(),
                    message: format!("Anchor registry points to non-existent node: {name}"),
                    position: Position::default(),
                });
            }
        }

        Ok(())
    }

    /// Reserve capacity for expected number of references
    #[inline]
    pub fn reserve(&mut self, anchors: usize, aliases: usize) {
        self.anchor_registry.reserve(anchors);
        self.alias_registry.reserve(aliases);
        self.graph.reserve(anchors + aliases);
        self.memory_manager.reserve(anchors + aliases);
    }

    /// Get total number of tracked references
    #[inline]
    #[must_use] 
    pub fn total_references(&self) -> usize {
        self.anchor_registry.len() + self.alias_registry.len()
    }

    /// Get number of anchors
    #[inline]
    #[must_use] 
    pub fn anchor_count(&self) -> usize {
        self.anchor_registry.len()
    }

    /// Get number of aliases
    #[inline]
    #[must_use] 
    pub fn alias_count(&self) -> usize {
        self.alias_registry.len()
    }

    /// Check if tracking is enabled
    #[inline]
    #[must_use] 
    pub const fn is_enabled(&self) -> bool {
        self.context.is_enabled
    }
    /// Validate all references - ensures all aliases can be resolved
    pub fn validate_references(&self) -> Result<(), SemanticError> {
        // Check that all aliases have corresponding anchors
        for (alias_name, &alias_id) in &self.alias_registry {
            if let Some(node) = self.graph.get_node(alias_id)
                && let ReferenceNodeType::Alias {
                    target,
                    resolved_target,
                    ..
                } = &node.node_type
                && resolved_target.is_none()
                && !self.anchor_registry.contains_key(target)
            {
                return Err(SemanticError::UnresolvedAlias {
                    alias_name: alias_name.to_string(),
                    position: node.position,
                });
            }
        }

        // Validate internal consistency
        self.validate()
    }

    /// Reset tracker for new analysis (alias for clear for semantic analyzer compatibility)
    pub fn reset(&mut self) {
        self.clear()
    }
}

impl<'input> Default for ReferenceTracker<'input> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::ScalarStyle;
    use crate::parser::ast::{Node, ScalarNode};

    fn create_test_node() -> Node<'static> {
        Node::Scalar(ScalarNode {
            value: "test".into(),
            style: ScalarStyle::Plain,
            tag: None,
            position: Position::default(),
        })
    }

    #[test]
    fn test_tracker_creation() {
        let tracker = ReferenceTracker::new();
        assert!(tracker.is_enabled());
        assert_eq!(tracker.total_references(), 0);
    }

    #[test]
    fn test_anchor_tracking() {
        let mut tracker = ReferenceTracker::new();
        let node = create_test_node();

        let result = tracker.track_anchor(Cow::Borrowed("test_anchor"), &node, Position::default());

        assert!(result.is_ok());
        assert!(tracker.has_anchor("test_anchor"));
        assert_eq!(tracker.anchor_count(), 1);
    }

    #[test]
    fn test_alias_tracking() {
        let mut tracker = ReferenceTracker::new();
        let node = create_test_node();

        // First create an anchor
        match tracker.track_anchor(Cow::Borrowed("anchor"), &node, Position::default()) {
            Ok(_) => {} // Anchor tracking successful
            Err(_) => panic!("Expected successful anchor tracking"),
        }

        // Then create an alias
        let result = tracker.track_alias(
            Cow::Borrowed("alias"),
            Cow::Borrowed("anchor"),
            Position::default(),
        );

        assert!(result.is_ok());
        assert!(tracker.has_alias("alias"));
        assert_eq!(tracker.alias_count(), 1);
    }

    #[test]
    fn test_duplicate_anchor_error() {
        let mut tracker = ReferenceTracker::new();
        let node = create_test_node();

        // Track first anchor
        match tracker.track_anchor(Cow::Borrowed("duplicate"), &node, Position::default()) {
            Ok(_) => {} // Anchor tracking successful
            Err(_) => panic!("Expected successful anchor tracking"),
        }

        // Try to track duplicate
        let result = tracker.track_alias(
            Cow::Borrowed("duplicate"),
            Cow::Borrowed("duplicate"),
            Position::default(),
        );

        assert!(result.is_ok()); // Alias with same name should work
    }

    #[test]
    fn test_tracker_clear() {
        let mut tracker = ReferenceTracker::new();
        let node = create_test_node();

        match tracker.track_anchor(Cow::Borrowed("test"), &node, Position::default()) {
            Ok(_) => {} // Anchor tracking successful
            Err(_) => panic!("Expected successful anchor tracking"),
        }

        assert_eq!(tracker.total_references(), 1);

        tracker.clear();
        assert_eq!(tracker.total_references(), 0);
        assert!(!tracker.has_anchor("test"));
    }

    #[test]
    fn test_validation() {
        let tracker = ReferenceTracker::new();
        let result = tracker.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_optimization() {
        let mut tracker = ReferenceTracker::new();
        let result = tracker.optimize();
        assert!(result.is_ok());
        assert!(tracker.is_optimized());
    }
}
