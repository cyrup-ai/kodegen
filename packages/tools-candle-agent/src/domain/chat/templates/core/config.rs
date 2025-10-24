//! Template configuration and auxiliary types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Template configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Maximum size in bytes for a single template
    pub max_template_size: usize,
    /// Maximum number of variables allowed in a template
    pub max_variables: usize,
    /// Whether to allow templates to include other templates
    pub allow_nested_templates: bool,
    /// Whether to cache compiled templates for reuse
    pub cache_compiled: bool,
    /// Whether to apply optimizations to templates during compilation
    pub optimize_templates: bool,
    /// Security restrictions for template execution
    pub security_mode: SecurityMode,
}

/// Security mode for templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityMode {
    /// No external access, limited functions
    Strict,
    /// Standard functions allowed
    Normal,
    /// Most functions allowed
    Relaxed,
    /// All functions allowed
    Disabled,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            max_template_size: 1024 * 1024, // 1MB
            max_variables: 1000,
            allow_nested_templates: true,
            cache_compiled: true,
            optimize_templates: true,
            security_mode: SecurityMode::Normal,
        }
    }
}

/// Template example for documentation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateExample {
    /// Name of the template example
    pub name: String,
    /// Description of what the template demonstrates
    pub description: String,
    /// Input variables and their example values
    pub input_variables: HashMap<String, String>,
    /// Expected output when template is rendered with the input variables
    pub expected_output: String,
}

/// Template tag for categorization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateTag {
    /// Name of the tag
    pub name: String,
    /// Optional color for UI display
    pub color: Option<String>,
    /// Optional description of the tag's purpose
    pub description: Option<String>,
}

impl TemplateTag {
    /// Create a new template tag with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: None,
            description: None,
        }
    }

    /// Set the color for this tag
    #[must_use]
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the description for this tag
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
