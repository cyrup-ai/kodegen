//! Validation rules and rule sets

use super::context::ValidationContext;
use super::warnings::ValidationWarning;
use crate::parser::ast::Node;
use crate::semantic::{AnalysisContext, SemanticError};

/// Trait for validation rules
pub trait ValidationRule<'input>: std::fmt::Debug {
    /// Name of the validation rule
    fn name(&self) -> &str;

    /// Description of what this rule validates
    fn description(&self) -> &str;

    /// Apply the rule to a node
    fn validate(
        &self,
        node: &Node<'input>,
        context: &mut ValidationContext,
        analysis_context: &AnalysisContext<'input>,
    ) -> Result<Vec<ValidationWarning<'input>>, SemanticError>;

    /// Check if this rule is applicable to the given node
    fn is_applicable(&self, node: &Node<'input>) -> bool;
}

/// Set of validation rules for different validation levels
#[derive(Debug)]
pub struct ValidationRuleSet<'input> {
    pub structural_rules: Vec<Box<dyn ValidationRule<'input>>>,
    pub semantic_rules: Vec<Box<dyn ValidationRule<'input>>>,
    pub constraint_rules: Vec<Box<dyn ValidationRule<'input>>>,
    pub custom_rules: Vec<Box<dyn ValidationRule<'input>>>,
}

impl<'input> Default for ValidationRuleSet<'input> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'input> ValidationRuleSet<'input> {
    /// Create a new empty rule set
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            structural_rules: Vec::new(),
            semantic_rules: Vec::new(),
            constraint_rules: Vec::new(),
            custom_rules: Vec::new(),
        }
    }

    /// Add a structural rule
    #[inline]
    pub fn add_structural_rule(&mut self, rule: Box<dyn ValidationRule<'input>>) {
        self.structural_rules.push(rule);
    }

    /// Add a semantic rule
    #[inline]
    pub fn add_semantic_rule(&mut self, rule: Box<dyn ValidationRule<'input>>) {
        self.semantic_rules.push(rule);
    }

    /// Add a constraint rule
    #[inline]
    pub fn add_constraint_rule(&mut self, rule: Box<dyn ValidationRule<'input>>) {
        self.constraint_rules.push(rule);
    }

    /// Add a custom rule
    #[inline]
    pub fn add_custom_rule(&mut self, rule: Box<dyn ValidationRule<'input>>) {
        self.custom_rules.push(rule);
    }

    /// Get all rules in execution order
    #[inline]
    pub fn all_rules(&self) -> impl Iterator<Item = &Box<dyn ValidationRule<'input>>> {
        self.structural_rules
            .iter()
            .chain(self.semantic_rules.iter())
            .chain(self.constraint_rules.iter())
            .chain(self.custom_rules.iter())
    }
}
