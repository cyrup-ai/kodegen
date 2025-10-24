//! Type definitions for reference tracking system
//!
//! Contains all core types, enums, and structs used throughout the reference tracking module.
//! Optimized for zero-allocation and blazing-fast performance.

use crate::lexer::Position;
use crate::parser::ast::Node;
use std::borrow::Cow;
use std::collections::HashSet;

/// Reference ID for efficient lookup - zero-allocation identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReferenceId(pub usize);

/// Reference node in the graph
#[derive(Debug, Clone)]
pub struct ReferenceNode<'input> {
    pub id: ReferenceId,
    pub name: Cow<'input, str>,
    pub node_type: ReferenceNodeType<'input>,
    pub position: Position,
    pub reference_path: Vec<String>,
}

/// Types of reference nodes
#[derive(Debug, Clone)]
pub enum ReferenceNodeType<'input> {
    Anchor {
        name: Cow<'input, str>,
        value: &'input Node<'input>,
        anchor_type: AnchorType,
    },
    Alias {
        target: Cow<'input, str>,
        resolved_target: Option<ReferenceId>,
        alias_type: AliasType,
    },
    Document {
        root: &'input Node<'input>,
        document_index: usize,
    },
    Mapping {
        key_count: usize,
        has_duplicate_keys: bool,
    },
    Sequence {
        element_count: usize,
        element_types: Vec<ReferenceNodeType<'input>>,
    },
    Scalar {
        value: Cow<'input, str>,
        scalar_type: ScalarType,
    },
}

/// Reference edge representing relationships
#[derive(Debug, Clone)]
pub struct ReferenceEdge {
    pub from: ReferenceId,
    pub to: ReferenceId,
    pub edge_type: EdgeType,
    pub metadata: EdgeMetadata,
}

/// Types of reference edges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    AliasReference,
    ChildRelation,
    DocumentLink,
    DependencyEdge,
    MergeRelation,
    CrossReference,
}

/// Edge metadata for additional information
#[derive(Debug, Clone)]
pub struct EdgeMetadata {
    pub weight: f64,
    pub priority: u8,
    pub is_critical: bool,
}

/// Graph metadata and statistics
#[derive(Debug, Clone)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub max_depth: usize,
    pub connected_components: usize,
    pub created_at: std::time::SystemTime,
    pub last_modified: std::time::SystemTime,
    pub optimization_level: u8,
}

/// Memory usage tracking
#[derive(Debug, Clone, Default)]
pub struct MemoryUsage {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub free_bytes: usize,
    pub fragmentation_ratio: f64,
    pub allocation_count: usize,
    pub deallocation_count: usize,
}

/// Cycle detection algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleDetectionAlgorithm {
    DepthFirstSearch,
    TarjanStronglyConnected,
    FloydWarshall,
    JohnsonAlgorithm,
}

/// Cycle detection result
#[derive(Debug, Clone)]
pub struct CycleDetectionResult {
    pub has_cycles: bool,
    pub cycles: Vec<Cycle>,
    pub performance_metrics: DetectionMetrics,
}

/// Detected cycle information
#[derive(Debug, Clone)]
pub struct Cycle {
    pub nodes: Vec<ReferenceId>,
    pub cycle_type: CycleType,
    pub severity: CycleSeverity,
    pub breaking_suggestions: Vec<CycleBreakingSuggestion>,
}

/// Types of cycles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleType {
    SelfReference,
    DirectCycle,
    IndirectCycle,
    ComplexCycle,
}

/// Cycle severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CycleSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Suggestion for breaking cycles
#[derive(Debug, Clone)]
pub struct CycleBreakingSuggestion {
    pub breaking_type: BreakingType,
    pub target_edge: Option<ReferenceEdge>,
    pub impact_assessment: ImpactAssessment,
    pub description: String,
}

/// Types of cycle breaking strategies
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakingType {
    RemoveEdge,
    AddIndirection,
    RestructureReference,
    ConvertToWeakReference,
}

/// Impact assessment for cycle breaking
#[derive(Debug, Clone)]
pub struct ImpactAssessment {
    pub confidence: f64,
    pub risk_level: RiskLevel,
    pub estimated_performance_gain: f64,
}

/// Risk levels for impact assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Anchor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorType {
    Standard,
    Merge,
    Override,
}

/// Alias types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasType {
    Simple,
    Merge,
    Override,
}

/// Scalar types for reference tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarType {
    String,
    Integer,
    Float,
    Boolean,
    Null,
    Timestamp,
    Binary,
}

/// Reference tracking statistics
#[derive(Debug, Clone)]
pub struct ReferenceStatistics {
    pub total_references: usize,
    pub resolved_references: usize,
    pub unresolved_references: usize,
    pub cycle_count: usize,
    pub max_reference_depth: usize,
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStatistics {
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f64,
    pub connected_components: usize,
    pub max_depth: usize,
    pub avg_degree: f64,
}

/// Optimization result
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub memory_saved: usize,
    pub references_removed: usize,
    pub performance_improvement: f64,
}

/// Detection performance metrics
#[derive(Debug, Clone)]
pub struct DetectionMetrics {
    pub detection_time_ms: u64,
    pub nodes_visited: usize,
    pub memory_usage: usize,
}

/// Tracking context for maintaining state
#[derive(Debug)]
pub struct TrackingContext {
    pub is_enabled: bool,
    pub max_depth: usize,
    pub current_depth: usize,
    pub visited_nodes: HashSet<ReferenceId>,
}

impl Default for GraphMetadata {
    #[inline]
    fn default() -> Self {
        let now = std::time::SystemTime::now();
        Self {
            node_count: 0,
            edge_count: 0,
            max_depth: 0,
            connected_components: 0,
            created_at: now,
            last_modified: now,
            optimization_level: 0,
        }
    }
}

impl std::fmt::Display for ReferenceId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ref_{}", self.0)
    }
}
