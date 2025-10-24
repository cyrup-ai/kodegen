//! Constraint checking for YAML semantic constraints

use super::context::ValidationContext;
use super::warnings::ValidationWarning;
use crate::parser::ast::Node;
use crate::semantic::tags::YamlType;
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
