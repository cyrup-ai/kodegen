//! Main document validator implementation

use super::{
    ConstraintChecker, StructureAnalyzer, ValidationContext, ValidationRuleSet, ValidationWarning,
    WarningSeverity, WarningType,
};
use crate::parser::ast::{Document, Node};
use crate::semantic::{AnalysisContext, SemanticError};

/// Comprehensive document validator with configurable rules
#[derive(Debug)]
pub struct DocumentValidator<'input> {
    pub validation_rules: ValidationRuleSet<'input>,
    pub constraint_checker: ConstraintChecker<'input>,
    pub structure_analyzer: StructureAnalyzer<'input>,
    pub validation_context: ValidationContext,
}

impl<'input> Default for DocumentValidator<'input> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'input> DocumentValidator<'input> {
    /// Create a new document validator with default settings
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            validation_rules: ValidationRuleSet::default(),
            constraint_checker: ConstraintChecker::new(),
            structure_analyzer: StructureAnalyzer::new(),
            validation_context: ValidationContext::new(),
        }
    }

    /// Create a validator with custom validation rules
    #[inline]
    #[must_use] 
    pub fn with_rules(validation_rules: ValidationRuleSet<'input>) -> Self {
        Self {
            validation_rules,
            constraint_checker: ConstraintChecker::new(),
            structure_analyzer: StructureAnalyzer::new(),
            validation_context: ValidationContext::new(),
        }
    }

    /// Create validator with specific configuration
    #[must_use] 
    pub fn with_config(config: &crate::semantic::SemanticConfig<'input>) -> Self {
        let mut validator = Self::new();

        // Configure based on strict mode
        if config.strict_mode {
            validator.validation_context.set_strict_mode(true);
        }

        // Add YAML version specific validation rules
        if let Some((major, minor)) = config.yaml_version {
            validator.validation_context.set_yaml_version(major, minor);
        }

        validator
    }

    /// Validate a complete YAML document
    pub fn validate_document(
        &mut self,
        document: &Document<'input>,
        analysis_context: &AnalysisContext<'input>,
    ) -> Result<Vec<ValidationWarning<'input>>, SemanticError> {
        self.validation_context.reset();
        let mut warnings = Vec::new();

        // Validate the document node
        if let Some(node) = &document.content {
            warnings.extend(self.validate_node(node, analysis_context)?);
        }

        // Generate validation statistics
        let stats = self.generate_statistics();

        // Add summary warning if issues found
        if stats.total_warnings > 0 {
            warnings.insert(0, ValidationWarning {
                warning_type: WarningType::StructuralIssue,
                message: format!(
                    "Document validation found {} issues ({} critical, {} high, {} medium, {} low, {} info)",
                    stats.total_warnings,
                    stats.critical_count,
                    stats.high_count,
                    stats.medium_count,
                    stats.low_count,
                    stats.info_count
                ),
                position: document.position,
                severity: if stats.critical_count > 0 {
                    WarningSeverity::Critical
                } else if stats.high_count > 0 {
                    WarningSeverity::High
                } else {
                    WarningSeverity::Medium
                },
                rule_name: "validation_summary".to_string(),
                suggestion: Some("Review and fix validation issues".to_string()),
                context: super::warnings::ValidationWarningContext {
                    node_path: vec!["document".to_string()],
                    node_type: None,
                    related_nodes: Vec::new(),
                    constraint_violated: None,
                    suggested_fix: None,
                },
            });
        }

        Ok(warnings)
    }

    /// Validate a single node recursively
    pub fn validate_node(
        &mut self,
        node: &Node<'input>,
        analysis_context: &AnalysisContext<'input>,
    ) -> Result<Vec<ValidationWarning<'input>>, SemanticError> {
        let mut warnings = Vec::new();

        // Check for depth limit
        if self.validation_context.is_depth_exceeded() {
            return Err(SemanticError::ValidationDepthExceeded {
                max_depth: self.validation_context.max_depth,
                current_depth: self.validation_context.current_depth,
                position: crate::lexer::Position::default(),
            });
        }

        // Check for circular references
        let node_id = node as *const _ as usize;
        if !self.validation_context.mark_visited(node_id) {
            return Err(SemanticError::CircularReference {
                alias_name: "unknown".to_string(),
                path: self.validation_context.current_path(),
                position: crate::lexer::Position::default(),
            });
        }

        // Apply validation rules
        for rule in self.validation_rules.all_rules() {
            if rule.is_applicable(node) {
                let rule_warnings =
                    rule.validate(node, &mut self.validation_context, analysis_context)?;
                warnings.extend(rule_warnings);
            }
        }

        // Check constraints
        warnings.extend(self.constraint_checker.check_constraints(
            node,
            &mut self.validation_context,
            analysis_context,
        )?);

        // Analyze structure
        self.structure_analyzer
            .analyze_structure(node, analysis_context);

        // Validate child nodes based on node type
        match node {
            Node::Sequence(seq) => {
                for (idx, item) in seq.items.iter().enumerate() {
                    self.validation_context.push_path(format!("[{idx}]"));
                    warnings.extend(self.validate_node(item, analysis_context)?);
                    self.validation_context.pop_path();
                }
            }
            Node::Mapping(map) => {
                for pair in &map.pairs {
                    // Validate key
                    self.validation_context.push_path("$key".to_string());
                    warnings.extend(self.validate_node(&pair.key, analysis_context)?);
                    self.validation_context.pop_path();

                    // Validate value
                    if let Node::Scalar(key_scalar) = &pair.key {
                        self.validation_context
                            .push_path(key_scalar.value.to_string());
                    } else {
                        self.validation_context.push_path("$value".to_string());
                    }
                    warnings.extend(self.validate_node(&pair.value, analysis_context)?);
                    self.validation_context.pop_path();
                }
            }
            _ => {} // Scalar and Alias nodes have no children
        }

        Ok(warnings)
    }

    /// Generate validation statistics
    #[must_use] 
    pub fn generate_statistics(&self) -> ValidationStatistics {
        ValidationStatistics {
            total_warnings: self.validation_context.warning_count,
            critical_count: 0, // Would be calculated from actual warnings
            high_count: 0,
            medium_count: 0,
            low_count: 0,
            info_count: 0,
            total_errors: self.validation_context.error_count,
            nodes_visited: self.validation_context.visited_nodes.len(),
            max_depth_reached: self.validation_context.current_depth,
            complexity_score: self.structure_analyzer.complexity_score(),
        }
    }
}

/// Validation statistics summary
#[derive(Debug, Clone, Default)]
pub struct ValidationStatistics {
    pub total_warnings: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
    pub total_errors: usize,
    pub nodes_visited: usize,
    pub max_depth_reached: usize,
    pub complexity_score: f32,
}
