//! Complexity metrics and optimization hints

use crate::lexer::Position;

/// Metrics for document complexity analysis
#[derive(Debug, Clone, Default)]
pub struct ComplexityMetrics {
    pub max_depth: usize,
    pub total_nodes: usize,
    pub scalar_count: usize,
    pub sequence_count: usize,
    pub mapping_count: usize,
    pub anchor_count: usize,
    pub alias_count: usize,
    pub tag_count: usize,
    pub longest_scalar: usize,
    pub largest_collection: usize,
}

impl ComplexityMetrics {
    /// Calculate a complexity score based on the metrics
    #[inline]
    #[must_use] 
    pub fn calculate_complexity_score(&self) -> f32 {
        let base_score = (self.total_nodes as f32).log2();
        let depth_factor = (self.max_depth as f32) * 0.5;
        let reference_factor = ((self.anchor_count + self.alias_count) as f32) * 0.3;
        let size_factor = (self.largest_collection as f32).log2() * 0.2;

        base_score + depth_factor + reference_factor + size_factor
    }

    /// Check if document is considered complex
    #[inline]
    #[must_use] 
    pub fn is_complex(&self) -> bool {
        self.calculate_complexity_score() > 10.0
            || self.max_depth > 10
            || self.total_nodes > 1000
            || self.largest_collection > 100
    }
}

/// Optimization hint for improving document structure
#[derive(Debug, Clone)]
pub struct OptimizationHint {
    pub optimization_type: OptimizationType,
    pub description: String,
    pub position: Position,
    pub impact: String,
    pub difficulty: OptimizationDifficulty,
    pub estimated_improvement: f32,
}

/// Types of optimizations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationType {
    RefactorDeepNesting,
    UseAnchorsForDuplication,
    SplitLargeCollections,
    SimplifyComplexMappings,
    ConsolidateScalars,
    RemoveRedundancy,
}

/// Difficulty of implementing an optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationDifficulty {
    Trivial,
    Easy,
    Medium,
    Hard,
    Complex,
}

impl std::fmt::Display for OptimizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RefactorDeepNesting => write!(f, "Refactor Deep Nesting"),
            Self::UseAnchorsForDuplication => write!(f, "Use Anchors for Duplication"),
            Self::SplitLargeCollections => write!(f, "Split Large Collections"),
            Self::SimplifyComplexMappings => write!(f, "Simplify Complex Mappings"),
            Self::ConsolidateScalars => write!(f, "Consolidate Scalars"),
            Self::RemoveRedundancy => write!(f, "Remove Redundancy"),
        }
    }
}

impl std::fmt::Display for OptimizationDifficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trivial => write!(f, "Trivial"),
            Self::Easy => write!(f, "Easy"),
            Self::Medium => write!(f, "Medium"),
            Self::Hard => write!(f, "Hard"),
            Self::Complex => write!(f, "Complex"),
        }
    }
}
