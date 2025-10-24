//! Template rendering engines
//!
//! Provides template rendering via compiled templates with full AST support.

use crate::domain::chat::templates::{
    compiler::TemplateCompiler,
    core::{
        ChatTemplate as CandleChatTemplate, TemplateContext as CandleTemplateContext,
        TemplateError as CandleTemplateError, TemplateResult as CandleTemplateResult,
    },
};

/// Template rendering engine trait
pub trait TemplateEngine: Send + Sync {
    /// Render a template with the given context
    ///
    /// # Errors
    ///
    /// Returns `CandleTemplateError` if template rendering fails
    fn render(
        &self,
        template: &CandleChatTemplate,
        context: &CandleTemplateContext,
    ) -> CandleTemplateResult<String>;

    /// Check if the engine supports the template format
    fn supports(&self, template: &CandleChatTemplate) -> bool;

    /// Get engine name
    fn name(&self) -> &'static str;
}

/// Production template engine using compiled templates with full AST support
pub struct CompiledEngine {
    compiler: TemplateCompiler,
}

impl CompiledEngine {
    /// Create a new compiled engine
    #[must_use]
    pub fn new() -> Self {
        Self {
            compiler: TemplateCompiler::new(),
        }
    }
}

impl Default for CompiledEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine for CompiledEngine {
    fn render(
        &self,
        template: &CandleChatTemplate,
        context: &CandleTemplateContext,
    ) -> CandleTemplateResult<String> {
        // Use cached compiled template if available
        if let Some(ref compiled) = template.compiled {
            return compiled.render(context);
        }

        // Otherwise compile on-the-fly
        let compiled = self.compiler.compile(template)?;
        compiled.render(context)
    }

    fn supports(&self, _template: &CandleChatTemplate) -> bool {
        true // Compiled engine supports all templates
    }

    fn name(&self) -> &'static str {
        "compiled"
    }
}

/// Template engine registry
pub struct EngineRegistry {
    engines: Vec<Box<dyn TemplateEngine>>,
}

impl EngineRegistry {
    /// Create a new engine registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    /// Create a registry with default engines
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(CompiledEngine::new()));
        registry
    }

    /// Register a new engine
    pub fn register(&mut self, engine: Box<dyn TemplateEngine>) {
        self.engines.push(engine);
    }

    /// Find the best engine for a template
    pub fn find_engine(&self, template: &CandleChatTemplate) -> Option<&dyn TemplateEngine> {
        self.engines
            .iter()
            .find(|engine| engine.supports(template))
            .map(AsRef::as_ref)
    }

    /// Render a template using the best available engine
    ///
    /// # Errors
    ///
    /// Returns `CandleTemplateError` if:
    /// - No suitable engine is found for the template
    /// - Template rendering fails
    pub fn render(
        &self,
        template: &CandleChatTemplate,
        context: &CandleTemplateContext,
    ) -> CandleTemplateResult<String> {
        match self.find_engine(template) {
            Some(engine) => engine.render(template, context),
            None => Err(CandleTemplateError::RenderError {
                message: "No suitable rendering engine found".to_string(),
            }),
        }
    }
}

impl Default for EngineRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}
