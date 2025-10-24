//! Structure analysis and pattern detection

use super::metrics::{ComplexityMetrics, OptimizationHint};
use crate::lexer::Position;
use crate::parser::ast::Node;
use crate::semantic::AnalysisContext;

/// Pattern detector trait for identifying structural patterns
pub trait PatternDetector<'input>: std::fmt::Debug {
    /// Name of the pattern this detector identifies
    fn pattern_name(&self) -> &str;

    /// Detect pattern in the given node
    fn detect(
        &self,
        node: &Node<'input>,
        context: &AnalysisContext<'input>,
    ) -> Option<PatternMatch<'input>>;
}

/// Represents a detected pattern in the document
#[derive(Debug, Clone)]
pub struct PatternMatch<'input> {
    pub pattern_name: String,
    pub positions: Vec<Position>,
    pub confidence: f32,
    pub impact: PatternImpact,
    pub description: String,
    pub optimization_hint: Option<OptimizationHint>,
    pub _phantom: std::marker::PhantomData<&'input ()>,
}

/// Impact of a detected pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternImpact {
    Positive,
    Neutral,
    Negative,
    Performance,
    Maintenance,
}

/// Structure analyzer for document complexity and patterns
#[derive(Debug)]
pub struct StructureAnalyzer<'input> {
    pub complexity_metrics: ComplexityMetrics,
    pub pattern_detectors: Vec<Box<dyn PatternDetector<'input>>>,
    pub optimization_hints: Vec<OptimizationHint>,
}

impl<'input> Default for StructureAnalyzer<'input> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'input> StructureAnalyzer<'input> {
    /// Create a new structure analyzer
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            complexity_metrics: ComplexityMetrics::default(),
            pattern_detectors: Vec::new(),
            optimization_hints: Vec::new(),
        }
    }

    /// Add a pattern detector
    #[inline]
    pub fn add_pattern_detector(&mut self, detector: Box<dyn PatternDetector<'input>>) {
        self.pattern_detectors.push(detector);
    }

    /// Analyze node structure and update metrics
    pub fn analyze_structure(&mut self, node: &Node<'input>, context: &AnalysisContext<'input>) {
        // Update complexity metrics based on node type
        match node {
            Node::Scalar(_) => {
                self.complexity_metrics.scalar_count += 1;
            }
            Node::Sequence(seq) => {
                self.complexity_metrics.sequence_count += 1;
                self.complexity_metrics.total_nodes += seq.items.len();
            }
            Node::Mapping(map) => {
                self.complexity_metrics.mapping_count += 1;
                self.complexity_metrics.total_nodes += map.pairs.len() * 2;
            }
            _ => {}
        }

        // Detect patterns
        for detector in &self.pattern_detectors {
            if let Some(pattern_match) = detector.detect(node, context)
                && let Some(hint) = pattern_match.optimization_hint
            {
                self.optimization_hints.push(hint);
            }
        }
    }

    /// Get current complexity score
    #[inline]
    #[must_use] 
    pub fn complexity_score(&self) -> f32 {
        self.complexity_metrics.calculate_complexity_score()
    }

    /// Get optimization hints
    #[inline]
    #[must_use] 
    pub fn get_optimization_hints(&self) -> &[OptimizationHint] {
        &self.optimization_hints
    }

    /// Reset analyzer state
    #[inline]
    pub fn reset(&mut self) {
        self.complexity_metrics = ComplexityMetrics::default();
        self.optimization_hints.clear();
    }
}
