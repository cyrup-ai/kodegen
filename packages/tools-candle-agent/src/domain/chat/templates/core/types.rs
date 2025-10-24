//! Core template metadata types

use serde::{Deserialize, Serialize};

/// Template information structure for metadata queries
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// Unique identifier for the template
    pub id: String,
    /// Human-readable name of the template
    pub name: String,
    /// Category classification for organizational purposes
    pub category: TemplateCategory,
    /// Size of the template content in bytes
    pub size: usize,
    /// Number of template variables that can be substituted
    pub variable_count: usize,
    /// Unix timestamp when the template was created
    pub created_at: i64,
    /// Unix timestamp when the template was last modified
    pub modified_at: i64,
    /// Version string for template versioning
    pub version: String,
}

/// Template category enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TemplateCategory {
    /// General chat conversation templates
    #[default]
    Chat,
    /// System-level templates for internal operations
    System,
    /// User-facing templates for user interactions
    User,
    /// Assistant response templates
    Assistant,
    /// Function call templates for structured operations
    Function,
    /// Tool usage templates for structured operations
    Tool,
    /// Context injection templates for conversation enhancement
    Context,
    /// Prompt templates for guiding AI behavior
    Prompt,
    /// Response formatting templates
    Response,
    /// User-defined custom template category
    Custom,
}

/// Variable type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariableType {
    /// String/text variable type
    String,
    /// Numeric variable type (integers and floats)
    Number,
    /// Boolean true/false variable type
    Boolean,
    /// Array/list variable type
    Array,
    /// Object/map variable type with key-value pairs
    Object,
    /// Any type - accepts all variable types
    Any,
}

/// Template variable definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Name of the template variable (used for substitution)
    pub name: String,
    /// Human-readable description of the variable's purpose
    pub description: String,
    /// Data type of the variable (string, number, boolean, etc.)
    pub var_type: VariableType,
    /// Default value to use if no value is provided
    pub default_value: Option<String>,
    /// Whether this variable must be provided for template rendering
    pub required: bool,
    /// Regular expression pattern for validating variable values
    pub validation_pattern: Option<String>,
    /// List of valid values for enumeration-type variables
    pub valid_values: Option<Vec<String>>,
    /// Minimum value for numeric variables
    pub min_value: Option<f64>,
    /// Maximum value for numeric variables
    pub max_value: Option<f64>,
}

bitflags::bitflags! {
    /// Template permissions using bitflags for zero-allocation checks
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct TemplatePermissions: u8 {
        /// Template can be read by users
        const READ = 1 << 0;
        /// Template can be modified by users
        const WRITE = 1 << 1;
        /// Template can be executed/rendered by users
        const EXECUTE = 1 << 2;
        /// Template can be shared with other users
        const SHARE = 1 << 3;
        /// Template can be deleted by users
        const DELETE = 1 << 4;
        /// All permissions enabled
        const ALL = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits() | Self::SHARE.bits() | Self::DELETE.bits();
    }
}

impl Default for TemplatePermissions {
    #[inline]
    fn default() -> Self {
        Self::ALL
    }
}

/// Template metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Unique identifier for the template
    pub id: String,
    /// Human-readable name of the template
    pub name: String,
    /// Description of what the template does
    pub description: String,
    /// Author or creator of the template
    pub author: String,
    /// Version string of the template
    pub version: String,
    /// Category classification for the template
    pub category: TemplateCategory,
    /// Tags associated with the template for search/filtering
    pub tags: Vec<String>,
    /// Timestamp when the template was created
    pub created_at: u64,
    /// Timestamp when the template was last modified
    pub modified_at: u64,
    /// Number of times this template has been used
    pub usage_count: u64,
    /// User rating of the template (0.0 to 5.0)
    pub rating: f64,
    /// Permission settings for the template
    pub permissions: TemplatePermissions,
}
