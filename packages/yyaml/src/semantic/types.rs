//! Core types and result structures for semantic analysis
//!
//! Provides fundamental types used throughout semantic analysis including
//! results, metrics, and analysis state types.

use crate::parser::ast::Document;
use std::time::Duration;

/// Comprehensive semantic analysis results
#[derive(Debug, Clone)]
pub struct SemanticResult<'input> {
    /// Documents after semantic processing
    pub documents: Vec<Document<'input>>,

    /// Performance and processing metrics
    pub metrics: AnalysisMetrics,

    /// Warnings encountered during processing (non-fatal issues)
    pub warnings: Vec<SemanticWarning>,
}

/// Performance metrics for semantic analysis
#[derive(Debug, Clone, Default)]
pub struct AnalysisMetrics {
    /// Total processing time for semantic analysis
    pub processing_time: Duration,

    /// Number of documents processed
    pub documents_processed: usize,

    /// Number of anchors successfully resolved
    pub anchors_resolved: usize,

    /// Number of aliases successfully resolved
    pub aliases_resolved: usize,

    /// Number of tags successfully resolved
    pub tags_resolved: usize,

    /// Number of circular references detected
    pub cycles_detected: usize,
}

/// Warnings encountered during semantic analysis
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticWarning {
    /// Unused anchor definition
    UnusedAnchor {
        anchor_name: String,
        position: crate::lexer::Position,
    },

    /// Deprecated tag usage
    DeprecatedTag {
        tag: String,
        suggested_replacement: Option<String>,
        position: crate::lexer::Position,
    },

    /// Potentially inefficient structure
    InefficiencyWarning {
        description: String,
        suggestion: String,
        position: crate::lexer::Position,
    },

    /// Custom validation warning
    CustomValidationWarning {
        validator_name: String,
        message: String,
        position: crate::lexer::Position,
    },
}

impl<'input> SemanticResult<'input> {
    /// Create a new semantic result
    #[must_use] 
    pub fn new(documents: Vec<Document<'input>>) -> Self {
        Self {
            documents,
            metrics: AnalysisMetrics::default(),
            warnings: Vec::new(),
        }
    }

    /// Create semantic result with metrics
    #[must_use] 
    pub const fn with_metrics(documents: Vec<Document<'input>>, metrics: AnalysisMetrics) -> Self {
        Self {
            documents,
            metrics,
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the result
    pub fn add_warning(&mut self, warning: SemanticWarning) {
        self.warnings.push(warning);
    }

    /// Get the primary document (first document in the result)
    #[must_use] 
    pub fn primary_document(&self) -> Option<&Document<'input>> {
        self.documents.first()
    }

    /// Check if processing completed successfully (no warnings)
    #[must_use] 
    pub const fn is_clean(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Get processing summary
    #[must_use] 
    pub const fn summary(&self) -> ProcessingSummary {
        ProcessingSummary {
            documents_count: self.documents.len(),
            warnings_count: self.warnings.len(),
            processing_time: self.metrics.processing_time,
            anchors_resolved: self.metrics.anchors_resolved,
            aliases_resolved: self.metrics.aliases_resolved,
            tags_resolved: self.metrics.tags_resolved,
            cycles_detected: self.metrics.cycles_detected,
        }
    }
}

/// Summary of processing results for quick assessment
#[derive(Debug, Clone, Copy)]
pub struct ProcessingSummary {
    pub documents_count: usize,
    pub warnings_count: usize,
    pub processing_time: Duration,
    pub anchors_resolved: usize,
    pub aliases_resolved: usize,
    pub tags_resolved: usize,
    pub cycles_detected: usize,
}

impl AnalysisMetrics {
    /// Create new metrics with processing time
    #[must_use] 
    pub fn with_time(processing_time: Duration) -> Self {
        Self {
            processing_time,
            ..Default::default()
        }
    }

    /// Record anchor resolution
    pub const fn record_anchor_resolution(&mut self) {
        self.anchors_resolved += 1;
    }

    /// Record alias resolution
    pub const fn record_alias_resolution(&mut self) {
        self.aliases_resolved += 1;
    }

    /// Record tag resolution
    pub const fn record_tag_resolution(&mut self) {
        self.tags_resolved += 1;
    }

    /// Record cycle detection
    pub const fn record_cycle_detection(&mut self) {
        self.cycles_detected += 1;
    }

    /// Update document count
    pub const fn set_documents_processed(&mut self, count: usize) {
        self.documents_processed = count;
    }

    /// Calculate processing rate (nodes per second)
    #[must_use] 
    pub fn processing_rate(&self) -> f64 {
        if self.processing_time.as_secs_f64() > 0.0 {
            let total_operations =
                self.anchors_resolved + self.aliases_resolved + self.tags_resolved;
            total_operations as f64 / self.processing_time.as_secs_f64()
        } else {
            0.0
        }
    }

    /// Check if metrics indicate efficient processing
    #[must_use] 
    pub fn is_efficient(&self) -> bool {
        // Consider processing efficient if:
        // - No cycles detected
        // - Processing rate > 1000 operations/second
        self.cycles_detected == 0 && self.processing_rate() > 1000.0
    }
}

impl SemanticWarning {
    /// Get the position associated with this warning
    #[must_use] 
    pub const fn position(&self) -> crate::lexer::Position {
        match self {
            Self::UnusedAnchor { position, .. } => *position,
            Self::DeprecatedTag { position, .. } => *position,
            Self::InefficiencyWarning { position, .. } => *position,
            Self::CustomValidationWarning { position, .. } => *position,
        }
    }

    /// Get human-readable warning message
    #[must_use] 
    pub fn message(&self) -> String {
        match self {
            Self::UnusedAnchor { anchor_name, .. } => {
                format!("Unused anchor definition: '{anchor_name}'")
            }
            Self::DeprecatedTag {
                tag,
                suggested_replacement,
                ..
            } => {
                if let Some(replacement) = suggested_replacement {
                    format!("Deprecated tag '{tag}', consider using '{replacement}'")
                } else {
                    format!("Deprecated tag '{tag}'")
                }
            }
            Self::InefficiencyWarning {
                description,
                suggestion,
                ..
            } => {
                format!("{description} (Suggestion: {suggestion})")
            }
            Self::CustomValidationWarning {
                validator_name,
                message,
                ..
            } => {
                format!("[{validator_name}] {message}")
            }
        }
    }

    /// Create an unused anchor warning
    #[must_use] 
    pub const fn unused_anchor(anchor_name: String, position: crate::lexer::Position) -> Self {
        Self::UnusedAnchor {
            anchor_name,
            position,
        }
    }

    /// Create a deprecated tag warning
    #[must_use] 
    pub const fn deprecated_tag(
        tag: String,
        suggested_replacement: Option<String>,
        position: crate::lexer::Position,
    ) -> Self {
        Self::DeprecatedTag {
            tag,
            suggested_replacement,
            position,
        }
    }

    /// Create an inefficiency warning
    #[must_use] 
    pub const fn inefficiency_warning(
        description: String,
        suggestion: String,
        position: crate::lexer::Position,
    ) -> Self {
        Self::InefficiencyWarning {
            description,
            suggestion,
            position,
        }
    }

    /// Create a custom validation warning
    #[must_use] 
    pub const fn custom_validation_warning(
        validator_name: String,
        message: String,
        position: crate::lexer::Position,
    ) -> Self {
        Self::CustomValidationWarning {
            validator_name,
            message,
            position,
        }
    }
}

impl std::fmt::Display for SemanticWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

/// Convenience type alias for semantic analysis results
pub type AnalysisResult<T> = Result<T, crate::semantic::error::SemanticError>;

/// Trait for types that can provide analysis metrics
pub trait HasMetrics {
    fn metrics(&self) -> &AnalysisMetrics;
    fn metrics_mut(&mut self) -> &mut AnalysisMetrics;
}

impl<'input> HasMetrics for SemanticResult<'input> {
    fn metrics(&self) -> &AnalysisMetrics {
        &self.metrics
    }

    fn metrics_mut(&mut self) -> &mut AnalysisMetrics {
        &mut self.metrics
    }
}

/// Trait for types that can collect warnings
pub trait WarningCollector {
    fn add_warning(&mut self, warning: SemanticWarning);
    fn warnings(&self) -> &[SemanticWarning];
    fn clear_warnings(&mut self);
}

impl<'input> WarningCollector for SemanticResult<'input> {
    fn add_warning(&mut self, warning: SemanticWarning) {
        self.warnings.push(warning);
    }

    fn warnings(&self) -> &[SemanticWarning] {
        &self.warnings
    }

    fn clear_warnings(&mut self) {
        self.warnings.clear();
    }
}
