//! Reference tracking module for YAML document relationships
//!
//! This module provides blazing-fast, zero-allocation reference tracking for YAML
//! documents with comprehensive cycle detection, memory optimization, and statistics.
//! Designed for production-grade performance with clean separation of concerns.

// Internal modules with focused responsibilities
mod cycle_detection;
mod graph;
mod memory;
mod statistics;
mod tracker;
mod types;

// Public re-exports for clean API
pub use cycle_detection::{CycleDetectionResult, CycleDetector};
pub use graph::ReferenceGraph;
pub use memory::MemoryManager;
pub use statistics::{StatisticsCollector, StatisticsReport};
pub use tracker::ReferenceTracker;
pub use types::{
    AliasType, AnchorType, BreakingType, Cycle, CycleDetectionAlgorithm, CycleSeverity, CycleType,
    DetectionMetrics, EdgeMetadata, EdgeType, GraphMetadata, GraphStatistics, ImpactAssessment,
    MemoryUsage, OptimizationResult, ReferenceEdge, ReferenceId, ReferenceNode, ReferenceNodeType,
    ReferenceStatistics, RiskLevel, ScalarType, TrackingContext,
};

/// Create a new reference tracker with optimized default settings
#[inline]
#[must_use] 
pub fn new_tracker<'input>() -> ReferenceTracker<'input> {
    ReferenceTracker::new()
}

/// Create a new reference tracker with custom capacity reservations
#[inline]
#[must_use] 
pub fn new_tracker_with_capacity<'input>(
    anchors: usize,
    aliases: usize,
) -> ReferenceTracker<'input> {
    let mut tracker = ReferenceTracker::new();
    tracker.reserve(anchors, aliases);
    tracker
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all main types are accessible
        let _tracker = ReferenceTracker::new();
        let _detector = CycleDetector::new();
        let _graph = ReferenceGraph::new();
        let _memory = MemoryManager::new();
        let _stats = StatisticsCollector::new();
    }

    #[test]
    fn test_convenience_functions() {
        let _tracker1 = new_tracker();
        let _tracker2 = new_tracker_with_capacity(100, 50);
    }
}
