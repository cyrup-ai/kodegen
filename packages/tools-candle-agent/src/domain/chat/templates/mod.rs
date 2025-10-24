//! Chat template system - Modular, high-performance template management
//!
//! This module provides a comprehensive template system for chat applications with
//! zero-allocation, lock-free architecture optimized for high-throughput scenarios.

use std::collections::HashMap;

pub mod cache;
pub mod compiler;
pub mod core;
pub mod engines;
pub mod filters;
pub mod manager;
pub mod parser;

// Re-export core types for convenience
pub use core::{
    ChatTemplate, CompiledTemplate, TemplateAst, TemplateCategory, TemplateConfig, TemplateContext,
    TemplateError, TemplateExample, TemplateInfo, TemplateMetadata, TemplateResult, TemplateTag,
    TemplateValue,
};
// Candle-prefixed aliases for compatibility
pub use core::{
    ChatTemplate as CandleChatTemplate, CompiledTemplate as CandleCompiledTemplate,
    TemplateAst as CandleTemplateAst, TemplateCategory as CandleTemplateCategory,
    TemplateError as CandleTemplateError, TemplateResult as CandleTemplateResult,
};
// Global template functions for convenience
// Duplicate HashMap import removed
use std::sync::Arc;

pub use compiler::TemplateCompiler;
// Re-export other important types
pub use manager::TemplateManager;
// Candle-prefixed aliases for managers and other components
pub use manager::TemplateManager as CandleTemplateManager;
pub use parser::TemplateParser;

/// Create a simple template
pub fn template(name: impl Into<String>, content: impl Into<String>) -> ChatTemplate {
    let template_name: String = name.into();
    let template_content: String = content.into();

    let metadata = core::TemplateMetadata {
        id: template_name.clone(),
        name: template_name,
        description: String::new(),
        author: String::new(),
        version: "1.0.0".to_string(),
        category: core::TemplateCategory::Chat,
        tags: Vec::new(),
        created_at: 0,
        modified_at: 0,
        usage_count: 0,
        rating: 0.0,
        permissions: core::TemplatePermissions::default(),
    };

    ChatTemplate::new(metadata, template_content, Arc::new([]))
}

/// Get global template manager
pub fn get_template_manager() -> &'static TemplateManager {
    use std::sync::OnceLock;
    static MANAGER: OnceLock<TemplateManager> = OnceLock::new();
    MANAGER.get_or_init(TemplateManager::new)
}

/// Store a template in global manager
///
/// # Errors
///
/// Returns `TemplateError` if template storage fails (see `TemplateManager::store`)
pub fn store_template(template: ChatTemplate) -> TemplateResult<()> {
    let manager = get_template_manager();
    manager.store(template)
}

/// Get a template from global manager
#[must_use]
pub fn get_template(name: &str) -> Option<ChatTemplate> {
    let manager = get_template_manager();
    manager.get(name).ok()
}

/// Render a template with variables
///
/// # Errors
///
/// Returns `TemplateError` if:
/// - Template with the given name is not found
/// - Template rendering fails
pub fn render_template<S: std::hash::BuildHasher>(
    name: &str,
    variables: &HashMap<String, String, S>,
) -> TemplateResult<String> {
    if let Some(template) = get_template(name) {
        template.render(variables)
    } else {
        Err(TemplateError::NotFound {
            name: name.to_string(),
        })
    }
}

/// Simple render function with string variables
///
/// # Errors
///
/// Returns `TemplateError` if template rendering fails (see `render_template`)
pub fn render_simple<S: std::hash::BuildHasher>(
    name: &str,
    variables: HashMap<&str, &str, S>,
) -> TemplateResult<String> {
    let arc_variables: HashMap<String, String> = variables
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    render_template(name, &arc_variables)
}

/// Create a simple render function
///
/// # Errors
///
/// Returns `TemplateError` if template rendering fails
pub fn render<S: std::hash::BuildHasher>(
    template: &ChatTemplate,
    variables: &HashMap<String, String, S>,
) -> TemplateResult<String> {
    template.render(variables)
}

/// Create template from string
#[must_use]
pub fn create_template(name: &str, content: &str) -> ChatTemplate {
    template(name, content)
}
