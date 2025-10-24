//! Template rendering context

use std::collections::HashMap;
use std::sync::Arc;

use super::error::TemplateResult;
use super::value::TemplateValue;

/// Type alias for template function
pub type TemplateFn = Arc<dyn Fn(&[TemplateValue]) -> TemplateResult<TemplateValue> + Send + Sync>;

/// Template context for rendering
#[derive(Clone)]
pub struct TemplateContext {
    /// Variables available during template rendering
    pub variables: HashMap<String, TemplateValue>,
    /// Functions available during template rendering
    pub functions: HashMap<String, TemplateFn>,
}

impl TemplateContext {
    /// Create a new empty template context
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    /// Add a variable to the context (builder pattern)
    #[must_use]
    pub fn with_variable(
        mut self,
        name: impl Into<String>,
        value: impl Into<TemplateValue>,
    ) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    /// Set a variable in the context (mutating)
    pub fn set_variable(&mut self, name: impl Into<String>, value: impl Into<TemplateValue>) {
        self.variables.insert(name.into(), value.into());
    }

    /// Get a variable value by name
    #[must_use]
    pub fn get_variable(&self, name: &str) -> Option<&TemplateValue> {
        self.variables.get(name)
    }

    /// Get all variables as a reference to the `HashMap`
    #[must_use]
    pub fn variables(&self) -> &HashMap<String, TemplateValue> {
        &self.variables
    }
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self::new()
    }
}
