//! Types and functionality for audio transcription
//!
//! This module provides types for handling audio transcription requests and responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Simple transcription result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcription {
    /// The transcribed text
    pub text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: Option<f64>,
    /// Language detected (ISO 639-1)
    pub language: Option<String>,
}

/// Request for transcribing audio content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRequest {
    /// Binary audio data to transcribe
    pub data: Vec<u8>,

    /// Original filename of the audio file
    pub filename: String,

    /// Language of the audio content (ISO 639-1)
    pub language: String,

    /// Optional prompt to guide the transcription
    pub prompt: Option<String>,

    /// Optional temperature for sampling (0.0 to 1.0)
    pub temperature: Option<f64>,

    /// Additional provider-specific parameters
    pub additional_params: Option<Value>,
}

impl TranscriptionRequest {
    /// Create a new transcription request with required fields
    pub fn new(data: Vec<u8>, filename: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            data,
            filename: filename.into(),
            language: language.into(),
            prompt: None,
            temperature: None,
            additional_params: None,
        }
    }

    /// Set an optional prompt to guide the transcription
    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set the temperature for sampling
    #[must_use]
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Add additional provider-specific parameters
    #[must_use]
    pub fn with_additional_params(mut self, params: Value) -> Self {
        self.additional_params = Some(params);
        self
    }
}

/// Response from a transcription operation
///
/// Wraps the actual transcription text with the original provider response
#[derive(Debug, Clone)]
pub struct TranscriptionResponse<T> {
    /// The transcribed text
    pub text: String,

    /// The original provider response
    pub response: T,
}

impl<T> TranscriptionResponse<T> {
    /// Create a new transcription response
    pub fn new(text: String, response: T) -> Self {
        Self { text, response }
    }

    /// Get the transcribed text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the original response
    pub fn into_inner(self) -> T {
        self.response
    }

    /// Map the inner response to a different type
    pub fn map<U, F>(self, f: F) -> TranscriptionResponse<U>
    where
        F: FnOnce(T) -> U,
    {
        TranscriptionResponse {
            text: self.text,
            response: f(self.response),
        }
    }
}
