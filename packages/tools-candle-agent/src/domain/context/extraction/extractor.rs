use std::fmt;
use std::marker::PhantomData;

use crate::async_stream;
use cyrup_sugars::prelude::MessageChunk;
use serde::de::DeserializeOwned;
use tokio_stream::{Stream, StreamExt};

use super::error::{_ExtractionResult as ExtractionResult, ExtractionError};
use crate::builders::completion::CompletionRequestBuilder;
use crate::capability::traits::TextToTextCapable;
use crate::domain::{
    chat::message::types::CandleMessageRole as MessageRole,
    completion::types::CandleCompletionParams as CompletionParams,
    context::chunks::{CandleCompletionChunk, FinishReason},
    prompt::CandlePrompt as Prompt,
};

/// Trait defining the core extraction interface
pub trait Extractor<T>: Send + Sync + fmt::Debug + Clone
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + MessageChunk + 'static,
{
    /// Get the system prompt for extraction
    fn system_prompt(&self) -> Option<&str>;

    /// Extract structured data from text with comprehensive error handling
    fn extract_from(&self, text: &str) -> impl Stream<Item = T>;

    /// Set system prompt for extraction guidance
    #[must_use]
    fn with_system_prompt(self, prompt: impl Into<String>) -> Self;
}

/// Implementation of the Extractor trait
#[derive(Clone)]
pub struct ExtractorImpl<T, P>
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + 'static,
    P: TextToTextCapable + Send + Sync + Clone,
{
    provider: P,
    system_prompt: Option<String>,
    _marker: PhantomData<T>,
}

impl<T, P> fmt::Debug for ExtractorImpl<T, P>
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + 'static,
    P: TextToTextCapable + Send + Sync + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtractorImpl")
            .field("provider", &"<TextToTextCapable>")
            .field("system_prompt", &self.system_prompt)
            .finish()
    }
}

impl<T, P> Extractor<T> for ExtractorImpl<T, P>
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + 'static + MessageChunk,
    P: TextToTextCapable + Send + Sync + Clone,
{
    fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    fn extract_from(&self, text: &str) -> impl Stream<Item = T> {
        let text = text.to_string();
        let provider = self.provider.clone();
        let system_prompt = self.system_prompt.clone().unwrap_or_else(|| {
            format!("Extract structured data from the following text. Return ONLY valid JSON matching the expected schema. Text: {text}")
        });

        async_stream::spawn_stream(move |tx| async move {
            let completion_request = match CompletionRequestBuilder::new()
                .system_prompt(system_prompt.clone())
                .build()
            {
                Ok(req) => req,
                Err(_e) => {
                    let _ = tx.send(T::default());
                    return;
                }
            };

            // Execute extraction asynchronously using tokio streams
            let model = &provider;
            let prompt = Prompt {
                content: completion_request.system_prompt,
                role: MessageRole::System,
            };
            let params = CompletionParams {
                temperature: completion_request.temperature,
                max_tokens: completion_request
                    .max_tokens
                    .and_then(|t| std::num::NonZeroU64::new(t.get())),
                n: std::num::NonZeroU8::MIN,
                stream: true,
                tools: None,
                additional_params: None,
            };

            // Get the stream and process chunks asynchronously
            let stream = model.prompt(prompt, &params);
            tokio::pin!(stream);

            // Process chunks to build response
            let mut full_response = String::new();
            let mut finish_reason = None;

            while let Some(chunk) = stream.next().await {
                match chunk {
                    CandleCompletionChunk::Text(text) => {
                        full_response.push_str(&text);
                    }
                    CandleCompletionChunk::Complete {
                        text,
                        finish_reason: reason,
                        ..
                    } => {
                        if !text.is_empty() {
                            full_response.push_str(&text);
                        }
                        finish_reason = reason;
                        break;
                    }
                    CandleCompletionChunk::Error(_err) => {
                        let _ = tx.send(T::default());
                        return;
                    }
                    _ => {}
                }
            }

            // Parse and send result
            if finish_reason == Some(FinishReason::Stop) || !full_response.is_empty() {
                match Self::parse_json_response(&full_response) {
                    Ok(result) => {
                        let _ = tx.send(result);
                    }
                    Err(_e) => {
                        let _ = tx.send(T::default());
                    }
                }
            } else {
                let _ = tx.send(T::default());
            }
        })
    }
}

impl<T, P> ExtractorImpl<T, P>
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + 'static + MessageChunk,
    P: TextToTextCapable + Send + Sync + Clone,
{
    /// Create new extractor with provider
    pub fn new_with_provider(provider: P) -> Self {
        Self {
            provider,
            system_prompt: None,
            _marker: PhantomData,
        }
    }

    /// Get the provider reference
    #[must_use]
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Parse JSON response
    ///
    /// # Errors
    ///
    /// Returns `ExtractionError` if:
    /// - Response is not valid JSON
    /// - JSON cannot be deserialized into type T
    pub fn parse_json_response(response: &str) -> ExtractionResult<T> {
        // First try to parse the whole response as JSON
        if let Ok(parsed) = serde_json::from_str::<T>(response) {
            return Ok(parsed);
        }

        // If that fails, try to find JSON in the response
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response
            .rfind('}')
            .map_or_else(|| response.len(), |i| i + 1);

        if json_start < json_end {
            let json_str = &response[json_start..json_end];
            serde_json::from_str(json_str).map_err(ExtractionError::from)
        } else {
            Err(ExtractionError::InvalidFormat {
                actual: response.to_string(),
            })
        }
    }
}
