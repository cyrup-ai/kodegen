//! Context extraction using completion providers for structured data extraction
//!
//! This module provides extraction capabilities for processing unstructured text
//! into structured data using AI completion providers.

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use tokio_stream::Stream;
use crate::async_stream;
use serde::de::DeserializeOwned;

use super::error::ExtractionError;
use crate::builders::completion::CompletionRequestBuilder;
use crate::domain::{
    chat::message::types::CandleMessageRole as MessageRole,
    completion::{
        types::CandleCompletionParams as CompletionParams,
    },
    context::chunks::{CandleCompletionChunk, FinishReason},
    prompt::CandlePrompt as Prompt,
};

/// Result type for extraction operations
type ExtractionResult<T> = Result<T, ExtractionError>;

/// Generic extractor trait for structured data extraction
pub trait Extractor<T>: Send + Sync + fmt::Debug + Clone
where
    T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + 'static,
{
    /// Get the system prompt for extraction
    fn system_prompt(&self) -> Option<&str>;

    /// Extract structured data from text with comprehensive error handling
    fn extract_from(&self, text: &str) -> impl Stream<Item = T>;

    /// Set system prompt for extraction guidance
    #[must_use]
    fn with_system_prompt(self, prompt: impl Into<String>) -> Self;
}

/// Document extractor for file-based extraction
#[derive(Clone)]
pub struct DocumentExtractor<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + 'static> {
    provider: Arc<dyn crate::capability::traits::TextToTextCapable + Send + Sync>,
    system_prompt: Option<String>,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + 'static> fmt::Debug for DocumentExtractor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DocumentExtractor")
            .field("provider", &"<dyn CompletionModel>")
            .field("system_prompt", &self.system_prompt)
            .finish()
    }
}

impl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + 'static> Extractor<T>
    for DocumentExtractor<T>
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
        let provider = Arc::clone(&self.provider);
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
            let model = provider.as_ref();
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

            // Get the stream and collect chunks
            let stream = model.prompt(prompt, &params);
            let chunks: Vec<_> = stream.collect();

            // Process chunks to build response
            let mut full_response = String::new();
            let mut finish_reason = None;

            for chunk in chunks {
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

impl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + 'static> DocumentExtractor<T> {
    /// Create new extractor with provider
    pub fn new_with_provider(provider: Arc<dyn crate::capability::traits::TextToTextCapable + Send + Sync>) -> Self {
        Self {
            provider,
            system_prompt: None,
            _marker: PhantomData,
        }
    }

    /// Get the provider reference
    pub fn provider(&self) -> &Arc<dyn crate::capability::traits::TextToTextCapable + Send + Sync> {
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
            serde_json::from_str(json_str).map_err(|e| ExtractionError::SerializationError(e.to_string()))
        } else {
            Err(ExtractionError::SerializationError(format!("Invalid JSON format: {}", response)))
        }
    }
}

/// Context extractor implementation
#[derive(Clone)]
pub struct ExtractorImpl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + 'static> {
    provider: Arc<dyn crate::capability::traits::TextToTextCapable + Send + Sync>,
    system_prompt: Option<String>,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + 'static> fmt::Debug for ExtractorImpl<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExtractorImpl")
            .field("provider", &"<dyn CompletionModel>")
            .field("system_prompt", &self.system_prompt)
            .finish()
    }
}

impl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + 'static> Extractor<T>
    for ExtractorImpl<T>
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
        let provider = Arc::clone(&self.provider);
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

            let model = provider.as_ref();
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

            let stream = model.prompt(prompt, &params);
            let chunks: Vec<_> = stream.collect();

            let mut full_response = String::new();
            let mut finish_reason = None;

            for chunk in chunks {
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

            if finish_reason == Some(FinishReason::Stop) || !full_response.is_empty() {
                match DocumentExtractor::<T>::parse_json_response(&full_response) {
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

impl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + 'static> ExtractorImpl<T> {
    /// Create new extractor with provider
    pub fn new_with_provider(provider: Arc<dyn crate::capability::traits::TextToTextCapable + Send + Sync>) -> Self {
        Self {
            provider,
            system_prompt: None,
            _marker: PhantomData,
        }
    }

    /// Get the provider reference
    pub fn provider(&self) -> &Arc<dyn crate::capability::traits::TextToTextCapable + Send + Sync> {
        &self.provider
    }
}

/// Batch extractor for processing multiple documents
#[derive(Clone)]
pub struct BatchExtractor<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + 'static> {
    extractor: ExtractorImpl<T>,
}

impl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + 'static> fmt::Debug for BatchExtractor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BatchExtractor")
            .field("extractor", &self.extractor)
            .finish()
    }
}

impl<T: DeserializeOwned + Send + Sync + fmt::Debug + Clone + Default + 'static> BatchExtractor<T> {
    /// Create a new batch extractor with provider
    pub fn new_with_provider(provider: Arc<dyn crate::capability::traits::TextToTextCapable + Send + Sync>) -> Self {
        Self {
            extractor: ExtractorImpl::new_with_provider(provider),
        }
    }

    /// Extract from multiple texts
    pub fn extract_batch(&self, texts: Vec<String>) -> impl Stream<Item = Vec<T>> {
        let extractor = self.extractor.clone();
        
        async_stream::spawn_stream(move |tx| async move {
            use tokio_stream::StreamExt;
            let mut results = Vec::with_capacity(texts.len());
            
            for text in texts {
                let mut extraction_stream = Box::pin(extractor.extract_from(&text));
                if let Some(result) = extraction_stream.next().await {
                    results.push(result);
                } else {
                    results.push(T::default());
                }
            }
            
            let _ = tx.send(results);
        })
    }
}
