//! Comprehensive semantic analysis and processing for YAML documents
//!
//! This module provides complete semantic processing including anchor/alias resolution,
//! tag resolution, document validation, and reference tracking with cycle detection.
//! Designed for zero-allocation, blazing-fast performance with complete YAML 1.2 compliance.

// Core submodules
pub mod analyzer;
pub mod context;
pub mod error;
pub mod optimization;
pub mod types;

// Existing semantic analysis modules
pub mod anchors;
pub mod references;
pub mod tags;
pub mod validation;

// Re-export core types for convenience
pub use analyzer::SemanticAnalyzer;
pub use context::{AnalysisContext, ProcessingPhase, SemanticConfig};
pub use error::{SemanticError, SemanticResult};
pub use optimization::{
    BufferSizeHints, CollectionCapacities, MemoryEstimate, OptimizationLevel, PerformanceConfig,
    SemanticOptimizations,
};
pub use types::{
    AnalysisMetrics, AnalysisResult, HasMetrics, ProcessingSummary, SemanticResult as Result,
    SemanticWarning, WarningCollector,
};

// Re-export existing semantic analysis functionality with specific exports to avoid conflicts
pub use anchors::{AnchorDefinition, AnchorRegistry, AnchorResolver};
pub use references::{ReferenceGraph, ReferenceTracker};
pub use tags::{TagRegistry as TagRegistryType, TagResolver};
pub use validation::{
    DocumentValidator, ValidationRule, ValidationRuleSet,
    WarningSeverity as ValidationWarningSeverity,
};

/// Default semantic analyzer instance with standard configuration
#[must_use] 
pub fn default_analyzer<'input>() -> SemanticAnalyzer<'input> {
    SemanticAnalyzer::new()
}

/// Create semantic analyzer optimized for speed
#[must_use] 
pub fn fast_analyzer<'input>() -> SemanticAnalyzer<'input> {
    SemanticAnalyzer::with_config(SemanticConfig::fast())
}

/// Create semantic analyzer optimized for strict validation
#[must_use] 
pub fn strict_analyzer<'input>() -> SemanticAnalyzer<'input> {
    SemanticAnalyzer::with_config(SemanticConfig::strict())
}

/// Convenience function for quick semantic analysis
pub fn analyze_stream<'input>(
    stream: crate::parser::ast::Stream<'input>,
) -> std::result::Result<types::SemanticResult<'input>, SemanticError> {
    let mut analyzer = default_analyzer();
    analyzer.analyze_stream(stream)
}

/// Convenience function for analyzing a single document
pub fn analyze_document<'input>(
    document: crate::parser::ast::Document<'input>,
) -> std::result::Result<crate::parser::ast::Document<'input>, SemanticError> {
    let mut analyzer = default_analyzer();
    analyzer.analyze_document(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Position;
    use crate::lexer::ScalarStyle;
    use crate::parser::ast::{Document, Node, ScalarNode, Stream};

    #[test]
    fn test_default_analyzer_creation() {
        let analyzer = default_analyzer();
        assert!(!analyzer.context().is_strict());
        assert!(analyzer.context().cycle_detection_enabled());
    }

    #[test]
    fn test_fast_analyzer_creation() {
        let analyzer = fast_analyzer();
        assert!(!analyzer.context().cycle_detection_enabled());
    }

    #[test]
    fn test_strict_analyzer_creation() {
        let analyzer = strict_analyzer();
        assert!(analyzer.context().is_strict());
    }

    #[test]
    fn test_analyze_simple_document() {
        let scalar = Node::Scalar(ScalarNode {
            value: "test".into(),
            style: ScalarStyle::Plain,
            tag: None,
            position: Position::default(),
        });

        let document = Document {
            content: Some(scalar),
            has_explicit_start: false,
            has_explicit_end: false,
            position: Position::default(),
        };

        let result = analyze_document(document);
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_empty_stream() {
        let stream = Stream { documents: vec![] };

        let result = analyze_stream(stream);
        assert!(result.is_ok());

        match result {
            Ok(analysis_result) => {
                assert_eq!(analysis_result.documents.len(), 0);
                assert_eq!(analysis_result.metrics.documents_processed, 0);
            }
            Err(_) => panic!("Expected successful analysis of empty stream"),
        }
    }

    #[test]
    fn test_semantic_config_builder() {
        let config = SemanticConfig::default()
            .with_yaml_version(1, 2)
            .with_strict_mode()
            .without_cycle_detection();

        assert_eq!(config.yaml_version, Some((1, 2)));
        assert!(config.strict_mode);
        assert!(!config.cycle_detection_enabled);
    }

    #[test]
    fn test_buffer_size_estimation() {
        let scalar = Node::Scalar(ScalarNode {
            value: "test".into(),
            style: ScalarStyle::Plain,
            tag: None,
            position: Position::default(),
        });

        let document = Document {
            content: Some(scalar),
            has_explicit_start: false,
            has_explicit_end: false,
            position: Position::default(),
        };

        let stream = Stream {
            documents: vec![document],
        };

        let hints = SemanticOptimizations::estimate_buffer_sizes(&stream);
        assert!(hints.estimated_nodes > 0);
    }

    #[test]
    fn test_analysis_metrics_recording() {
        let mut metrics = AnalysisMetrics::default();

        metrics.record_anchor_resolution();
        metrics.record_alias_resolution();
        metrics.record_tag_resolution();

        assert_eq!(metrics.anchors_resolved, 1);
        assert_eq!(metrics.aliases_resolved, 1);
        assert_eq!(metrics.tags_resolved, 1);
    }
}
