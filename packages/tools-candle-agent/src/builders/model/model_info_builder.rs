use std::num::NonZeroU32;
use crate::domain::model::{CandleModelInfo as ModelInfo};
use crate::domain::model::error::{CandleModelError as ModelError, CandleResult as Result};

/// Builder for creating ModelInfo instances
pub struct ModelInfoBuilder {
    provider_name: Option<&'static str>,
    name: Option<&'static str>,
    max_input_tokens: Option<NonZeroU32>,
    max_output_tokens: Option<NonZeroU32>,
    input_price: Option<f64>,
    output_price: Option<f64>,
    supports_vision: bool,
    supports_function_calling: bool,
    supports_streaming: bool,
    supports_embeddings: bool,
    requires_max_tokens: bool,
    supports_thinking: bool,
    optimal_thinking_budget: Option<u32>,
}

impl ModelInfoBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            provider_name: None,
            name: None,
            max_input_tokens: None,
            max_output_tokens: None,
            input_price: None,
            output_price: None,
            supports_vision: false,
            supports_function_calling: false,
            supports_streaming: false,
            supports_embeddings: false,
            requires_max_tokens: false,
            supports_thinking: false,
            optimal_thinking_budget: None,
        }
    }

    /// Set the provider name
    #[inline]
    pub fn provider_name(mut self, provider_name: &'static str) -> Self {
        self.provider_name = Some(provider_name);
        self
    }

    /// Set the model name
    #[inline]
    pub fn name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the maximum input tokens
    #[inline]
    pub fn max_input_tokens(mut self, tokens: u32) -> Self {
        self.max_input_tokens = NonZeroU32::new(tokens);
        self
    }

    /// Set the maximum output tokens
    #[inline]
    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        self.max_output_tokens = NonZeroU32::new(tokens);
        self
    }

    /// Set the pricing information
    #[inline]
    pub fn pricing(mut self, input_price: f64, output_price: f64) -> Self {
        self.input_price = Some(input_price);
        self.output_price = Some(output_price);
        self
    }

    /// Set whether the model supports vision
    #[inline]
    pub fn with_vision(mut self, supports: bool) -> Self {
        self.supports_vision = supports;
        self
    }

    /// Set whether the model supports function calling
    #[inline]
    pub fn with_function_calling(mut self, supports: bool) -> Self {
        self.supports_function_calling = supports;
        self
    }

    /// Set whether the model supports streaming
    #[inline]
    pub fn with_streaming(mut self, supports: bool) -> Self {
        self.supports_streaming = supports;
        self
    }

    /// Set whether the model supports embeddings
    #[inline]
    pub fn with_embeddings(mut self, supports: bool) -> Self {
        self.supports_embeddings = supports;
        self
    }

    /// Set whether max_tokens is required
    #[inline]
    pub fn requires_max_tokens(mut self, required: bool) -> Self {
        self.requires_max_tokens = required;
        self
    }

    /// Set thinking capabilities and budget
    #[inline]
    pub fn with_thinking(mut self, budget: u32) -> Self {
        self.supports_thinking = true;
        self.optimal_thinking_budget = Some(budget);
        self
    }

    /// Build the ModelInfo, validating the configuration
    pub fn build(self) -> Result<ModelInfo> {
        let model_info = ModelInfo {
            provider_name: self.provider_name.ok_or_else(|| {
                ModelError::InvalidConfiguration("provider_name is required".into())
            })?,
            name: self
                .name
                .ok_or_else(|| ModelError::InvalidConfiguration("name is required".into()))?,
            max_input_tokens: self.max_input_tokens,
            max_output_tokens: self.max_output_tokens,
            input_price: self.input_price,
            output_price: self.output_price,
            supports_vision: self.supports_vision,
            supports_function_calling: self.supports_function_calling,
            supports_streaming: self.supports_streaming,
            supports_embeddings: self.supports_embeddings,
            requires_max_tokens: self.requires_max_tokens,
            supports_thinking: self.supports_thinking,
            optimal_thinking_budget: self.optimal_thinking_budget,
            system_prompt_prefix: None,
            real_name: None,
            model_type: None,
            patch: None,
        };

        model_info.validate()?;
        Ok(model_info)
    }
}

impl Default for ModelInfoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience implementation for ModelInfo to access builder
impl ModelInfo {
    /// Create a new ModelInfo builder
    pub fn builder() -> ModelInfoBuilder {
        ModelInfoBuilder::new()
    }
}