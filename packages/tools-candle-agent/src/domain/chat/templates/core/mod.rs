//! Core template types and structures
//!
//! This module has been decomposed into focused submodules for better
//! maintainability while preserving the public API.

// Declare submodules
pub mod ast;
pub mod compiled;
pub mod config;
pub mod context;
pub mod error;
pub mod template;
pub mod types;
pub mod value;

// Re-export all public types to maintain backward compatibility
// Error types
pub use error::{TemplateError, TemplateResult};

// Core metadata types
pub use types::{
    TemplateCategory, TemplateInfo, TemplateMetadata, TemplatePermissions, TemplateVariable,
    VariableType,
};

// Value types
pub use value::TemplateValue;

// Context types
pub use context::{TemplateContext, TemplateFn};

// AST types
pub use ast::TemplateAst;

// Compiled template
pub use compiled::CompiledTemplate;

// Main template type
pub use template::ChatTemplate;

// Configuration types
pub use config::{SecurityMode, TemplateConfig, TemplateExample, TemplateTag};
