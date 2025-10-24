//! Main chat template type

use std::collections::HashMap;
use std::sync::Arc;

use super::compiled::CompiledTemplate;
use super::context::TemplateContext;
use super::error::{TemplateError, TemplateResult};
use super::types::{TemplateInfo, TemplateMetadata, TemplateVariable};
use super::value::TemplateValue;

/// Main chat template structure
#[derive(Debug, Clone)]
pub struct ChatTemplate {
    /// Template metadata and information
    pub metadata: TemplateMetadata,
    /// Raw template content string
    pub content: String,
    /// Template variables and their definitions
    pub variables: Arc<[TemplateVariable]>,
    /// Optional compiled template for faster rendering
    pub compiled: Option<CompiledTemplate>,
}

impl ChatTemplate {
    /// Create a new chat template
    #[must_use]
    pub fn new(
        metadata: TemplateMetadata,
        content: String,
        variables: Arc<[TemplateVariable]>,
    ) -> Self {
        Self {
            metadata,
            content,
            variables,
            compiled: None,
        }
    }

    /// Render the template with provided variables
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if template rendering fails or required variables are missing
    pub fn render<S: std::hash::BuildHasher>(
        &self,
        variables: &HashMap<String, String, S>,
    ) -> TemplateResult<String> {
        let mut context = TemplateContext::new();
        for (key, value) in variables {
            context.set_variable(key.clone(), TemplateValue::String(value.clone()));
        }

        if let Some(compiled) = &self.compiled {
            compiled.render(&context)
        } else {
            // Simple variable replacement for non-compiled templates
            let mut result = self.content.clone();
            for (key, value) in variables {
                result = result.replace(&format!("{{{{{key}}}}}"), value);
            }
            Ok(result)
        }
    }

    /// Get the template ID
    #[must_use]
    pub fn get_id(&self) -> &String {
        &self.metadata.id
    }

    /// Get the template name
    #[must_use]
    pub fn get_name(&self) -> &String {
        &self.metadata.name
    }

    /// Get the template content
    #[must_use]
    pub fn get_content(&self) -> &String {
        &self.content
    }

    /// Get the template variables
    #[must_use]
    pub fn get_variables(&self) -> &Arc<[TemplateVariable]> {
        &self.variables
    }

    /// Get template name (alias for `get_name` for compatibility)
    #[must_use]
    pub fn name(&self) -> &String {
        &self.metadata.name
    }

    /// Validate the template
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if:
    /// - Template name is empty
    /// - Template content is empty
    /// - Template validation checks fail
    pub fn validate(&self) -> TemplateResult<()> {
        // Basic validation
        if self.metadata.name.is_empty() {
            return Err(TemplateError::ParseError {
                message: "Template name cannot be empty".to_string(),
            });
        }

        if self.content.is_empty() {
            return Err(TemplateError::ParseError {
                message: "Template content cannot be empty".to_string(),
            });
        }

        // Additional validation can be added here
        Ok(())
    }

    /// Get template info
    #[must_use]
    pub fn info(&self) -> TemplateInfo {
        TemplateInfo {
            id: self.metadata.id.clone(),
            name: self.metadata.name.clone(),
            category: self.metadata.category,
            size: self.content.len(),
            variable_count: self.variables.len(),
            created_at: self.metadata.created_at.cast_signed(),
            modified_at: self.metadata.modified_at.cast_signed(),
            version: self.metadata.version.clone(),
        }
    }
}
