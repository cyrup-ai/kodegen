use std::sync::Arc;
use cyrup_sugars::OneOrMany;
use crate::domain::chat::templates::{
    CandleChatTemplate as ChatTemplate, 
    CandleTemplateCategory as TemplateCategory,
};
use crate::domain::chat::templates::core::{
    TemplateMetadata,
    TemplatePermissions,
    TemplateVariable,
    VariableType
};

/// Template builder struct
#[derive(Debug, Clone)]
pub struct TemplateBuilder {
    name: Option<String>,
    content: Option<String>,
    description: Option<String>,
    category: TemplateCategory,
    variables: OneOrMany<String>,
}

impl TemplateBuilder {
    /// Create a new template builder
    pub fn new() -> Self {
        Self {
            name: None,
            content: None,
            description: None,
            category: TemplateCategory::Chat,
            variables: OneOrMany::None,
        }
    }

    /// Set the template name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the template content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the template description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the template category
    pub fn category(mut self, category: TemplateCategory) -> Self {
        self.category = category;
        self
    }

    /// Add a variable to the template
    pub fn variable(mut self, var: impl Into<String>) -> Self {
        self.variables = self.variables.with_pushed(var.into());
        self
    }

    /// Build the chat template
    pub fn build(self) -> ChatTemplate {
        let name = self.name.unwrap_or_else(|| "untitled".to_string());
        let content = self.content.unwrap_or_else(|| "".to_string());
        let description = self.description.unwrap_or_else(|| "".to_string());

        let template_name: Arc<str> = Arc::from(name);
        let template_content: Arc<str> = Arc::from(content);

        let metadata = TemplateMetadata {
            id: template_name.clone(),
            name: template_name,
            description: Arc::from(description),
            author: Arc::from(""),
            version: Arc::from("1.0.0"),
            category: TemplateCategory::Chat,
            tags: Arc::new([]),
            created_at: 0,
            modified_at: 0,
            usage_count: 0,
            rating: 0.0,
            permissions: TemplatePermissions::default(),
        };

        let variables: Arc<[TemplateVariable]> = self
            .variables
            .into_iter()
            .map(|v| TemplateVariable {
                name: Arc::from(v),
                description: Arc::from(""),
                var_type: VariableType::String,
                default_value: None,
                required: false,
                validation_pattern: None,
                valid_values: None,
                min_value: None,
                max_value: None,
            })
            .collect::<Vec<_>>()
            .into();

        ChatTemplate::new(metadata, template_content, variables)
    }
}

impl Default for TemplateBuilder {
    fn default() -> Self {
        Self::new()
    }
}