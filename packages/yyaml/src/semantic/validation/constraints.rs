//! Constraint checking for YAML semantic constraints

use super::context::ValidationContext;
use super::warnings::{ValidationWarning, WarningSeverity, WarningType};
use crate::parser::ast::{Node, ScalarStyle};
use crate::semantic::tags::schema::SchemaProcessor;
use crate::semantic::tags::{types::SchemaType, YamlType};
use crate::semantic::{AnalysisContext, SemanticError};
use std::collections::HashMap;

/// Trait for constraint rules
pub trait ConstraintRule<'input>: std::fmt::Debug {
    /// Name of the constraint
    fn name(&self) -> &str;

    /// Check if constraint is satisfied
    fn check(
        &self,
        node: &Node<'input>,
        context: &mut ValidationContext,
        analysis_context: &AnalysisContext<'input>,
    ) -> Result<bool, SemanticError>;

    /// Generate warning if constraint is violated
    fn generate_warning(&self, node: &Node<'input>) -> ValidationWarning<'input>;
}

/// Type-specific constraints
#[derive(Debug, Clone)]
pub struct TypeConstraints {
    pub allowed_values: Option<Vec<String>>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub pattern: Option<String>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub required_keys: Option<Vec<String>>,
}

/// Structure-level constraints
#[derive(Debug, Clone)]
pub struct StructureConstraints {
    pub max_depth: usize,
    pub max_items: usize,
    pub max_keys: usize,
    pub disallow_duplicate_keys: bool,
    pub disallow_circular_refs: bool,
    pub require_explicit_tags: bool,
}

/// Constraint checker for YAML semantic constraints
#[derive(Debug)]
pub struct ConstraintChecker<'input> {
    pub type_constraints: HashMap<YamlType, TypeConstraints>,
    pub structure_constraints: StructureConstraints,
    pub custom_constraints: Vec<Box<dyn ConstraintRule<'input>>>,
}

impl<'input> Default for ConstraintChecker<'input> {
    fn default() -> Self {
        Self::new()
    }
}

/// Constraint that enforces canonical JSON scalars when JSON schema is active
#[derive(Debug)]
pub struct JsonScalarConstraint {
    json_processor: SchemaProcessor<'static>,
    core_processor: SchemaProcessor<'static>,
}

impl JsonScalarConstraint {
    #[must_use]
    pub fn new() -> Self {
        let mut json_processor = SchemaProcessor::<'static>::new();
        json_processor.set_schema(SchemaType::Json);
        let mut core_processor = SchemaProcessor::<'static>::new();
        core_processor.set_schema(SchemaType::Core);
        Self {
            json_processor,
            core_processor,
        }
    }
}

impl<'input> ConstraintRule<'input> for JsonScalarConstraint {
    fn name(&self) -> &str {
        "json_scalar_canonical_forms"
    }

    fn check(
        &self,
        node: &Node<'input>,
        _context: &mut ValidationContext,
        analysis_context: &AnalysisContext<'input>,
    ) -> Result<bool, SemanticError> {
        if analysis_context.schema_type() != SchemaType::Json {
            return Ok(true);
        }

        let Node::Scalar(scalar) = node else {
            return Ok(true);
        };

        if scalar.tag.is_some() || scalar.style != ScalarStyle::Plain {
            return Ok(true);
        }

        let value = scalar.value.as_ref().trim();
        if value.is_empty() {
            return Ok(true);
        }

        let json_type = self.json_processor.infer_scalar_type(value);
        if !matches!(json_type, YamlType::Str) {
            return Ok(true);
        }

        let core_type = self.core_processor.infer_scalar_type(value);
        let violates = matches!(core_type, YamlType::Null | YamlType::Bool | YamlType::Int | YamlType::Float);
        Ok(!violates)
    }

    fn generate_warning(&self, node: &Node<'input>) -> ValidationWarning<'input> {
        let value = match node {
            Node::Scalar(scalar) => scalar.value.as_ref().to_string(),
            _ => String::new(),
        };

        ValidationWarning {
            warning_type: WarningType::CompatibilityIssue,
            message: format!(
                "Scalar '{value}' is not a canonical JSON value under the JSON schema"
            ),
            position: node.position(),
            severity: WarningSeverity::High,
            rule_name: self.name().to_string(),
            suggestion: Some("Use canonical JSON scalars (null, true/false, or decimal numbers) or quote the value".to_string()),
            context: super::warnings::ValidationWarningContext {
                node_path: Vec::new(),
                node_type: None,
                related_nodes: Vec::new(),
                constraint_violated: Some(self.name().to_string()),
                suggested_fix: None,
            },
        }
    }
}

impl<'input> ConstraintChecker<'input> {
    /// Create a new constraint checker with default settings
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            type_constraints: HashMap::new(),
            structure_constraints: StructureConstraints::default(),
            custom_constraints: Vec::new(),
        }
    }

    /// Add a type constraint
    #[inline]
    pub fn add_type_constraint(&mut self, yaml_type: YamlType, constraints: TypeConstraints) {
        self.type_constraints.insert(yaml_type, constraints);
    }

    /// Add a custom constraint rule
    #[inline]
    pub fn add_custom_constraint(&mut self, constraint: Box<dyn ConstraintRule<'input>>) {
        self.custom_constraints.push(constraint);
    }

    /// Check all constraints for a node
    pub fn check_constraints(
        &self,
        node: &Node<'input>,
        context: &mut ValidationContext,
        analysis_context: &AnalysisContext<'input>,
    ) -> Result<Vec<ValidationWarning<'input>>, SemanticError> {
        let mut warnings = Vec::new();

        // Check custom constraints
        for constraint in &self.custom_constraints {
            if !constraint.check(node, context, analysis_context)? {
                warnings.push(constraint.generate_warning(node));
            }
        }

        Ok(warnings)
    }
}

impl Default for TypeConstraints {
    #[inline]
    fn default() -> Self {
        Self {
            allowed_values: None,
            min_value: None,
            max_value: None,
            pattern: None,
            min_length: None,
            max_length: None,
            required_keys: None,
        }
    }
}

impl Default for StructureConstraints {
    #[inline]
    fn default() -> Self {
        Self {
            max_depth: 1000,
            max_items: 1_000_000,
            max_keys: 100_000,
            disallow_duplicate_keys: true,
            disallow_circular_refs: true,
            require_explicit_tags: false,
        }
    }
}
