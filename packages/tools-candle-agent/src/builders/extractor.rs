//!
//! Extractor builder for structured data extraction with zero allocation.
//! Uses the real ExtractorImpl from domain/context/extraction with TextToTextCapable models.

use std::fmt;
use std::marker::PhantomData;

use cyrup_sugars::prelude::MessageChunk;
use serde::de::DeserializeOwned;

use crate::capability::registry::TextToTextModel;
use crate::domain::context::extraction::{Extractor, ExtractorImpl};

/// Extractor builder trait - elegant zero-allocation builder pattern
pub trait ExtractorBuilder<T>: Sized
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + MessageChunk + 'static,
{
    /// Set system prompt - EXACT syntax: .system_prompt("...")
    fn system_prompt(self, prompt: impl Into<String>) -> Self;

    /// Set instructions (alias for system_prompt) - EXACT syntax: .instructions("...")
    fn instructions(self, instructions: impl Into<String>) -> Self;

    /// Build extractor - EXACT syntax: .build()
    fn build(self) -> ExtractorImpl<T, TextToTextModel>;
}

/// Hidden implementation struct - zero-allocation builder state
pub struct ExtractorBuilderImpl<T>
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + MessageChunk + 'static,
{
    model: TextToTextModel,
    system_prompt: Option<String>,
    _marker: PhantomData<T>,
}

impl<T> ExtractorBuilder<T> for ExtractorBuilderImpl<T>
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + MessageChunk + 'static,
{
    fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.system_prompt = Some(instructions.into());
        self
    }

    fn build(self) -> ExtractorImpl<T, TextToTextModel> {
        let mut extractor = ExtractorImpl::new_with_provider(self.model);
        if let Some(prompt) = self.system_prompt {
            extractor = extractor.with_system_prompt(prompt);
        }
        extractor
    }
}

/// Entry point for extractor builder
///
/// # Example
/// ```no_run
/// use kodegen_candle_agent::builders::extractor::extractor;
/// use kodegen_candle_agent::capability::registry;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Deserialize, Serialize, Default)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let model = registry::get_text_to_text("phi4:latest")?;
///
/// let extractor = extractor::<Person>(model)
///     .system_prompt("Extract person information as JSON")
///     .build();
/// # Ok(())
/// # }
/// ```
pub fn extractor<T>(model: TextToTextModel) -> impl ExtractorBuilder<T>
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + MessageChunk + 'static,
{
    ExtractorBuilderImpl {
        model,
        system_prompt: None,
        _marker: PhantomData,
    }
}
