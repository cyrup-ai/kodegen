//! Completion response types and builders
//!
//! Contains response structures and builder patterns for completion functionality.

use std::borrow::Cow;

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};

use crate::domain::model::usage::CandleUsage;

/// A response from a text completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse<'a> {
    /// The generated completion text
    pub text: Cow<'a, str>,
    /// The model that generated the completion
    pub model: Cow<'a, str>,
    /// The provider that generated the completion (optional)
    pub provider: Option<Cow<'a, str>>,
    /// Token usage information (optional)
    pub usage: Option<CandleUsage>,
    /// The reason the completion finished (optional)
    pub finish_reason: Option<Cow<'a, str>>,
    /// Response time in milliseconds (optional)
    pub response_time_ms: Option<u64>,
    /// Generation time in milliseconds for performance tracking (optional)
    pub generation_time_ms: Option<u32>,
    /// Tokens per second throughput for performance tracking (optional)
    pub tokens_per_second: Option<f64>,
}

impl CompletionResponse<'_> {
    /// Get the completion text
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the model name
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the provider name if available
    #[must_use]
    pub fn provider(&self) -> Option<&str> {
        self.provider.as_deref()
    }

    /// Get the finish reason if available
    #[must_use]
    pub fn finish_reason(&self) -> Option<&str> {
        self.finish_reason.as_deref()
    }

    /// Get the response time in milliseconds if available
    #[must_use]
    pub fn response_time_ms(&self) -> Option<u64> {
        self.response_time_ms
    }

    /// Get the token usage if available
    #[must_use]
    pub fn usage(&self) -> Option<&CandleUsage> {
        self.usage.as_ref()
    }

    /// Get the generation time in milliseconds if available
    #[must_use]
    pub fn generation_time_ms(&self) -> Option<u32> {
        self.generation_time_ms
    }

    /// Get the tokens per second throughput if available
    #[must_use]
    pub fn tokens_per_second(&self) -> Option<f64> {
        self.tokens_per_second
    }

    /// Set the generation time in milliseconds for performance tracking
    pub fn set_generation_time_ms(&mut self, ms: u32) {
        self.generation_time_ms = Some(ms);
    }

    /// Set the tokens per second throughput for performance tracking
    pub fn set_tokens_per_second(&mut self, tps: f64) {
        self.tokens_per_second = Some(tps);
    }

    /// Get the number of tokens generated (output tokens) if available
    #[must_use]
    pub fn tokens_generated(&self) -> Option<u32> {
        self.usage.as_ref().map(|u| u.output_tokens)
    }
}

impl Default for CompletionResponse<'_> {
    fn default() -> Self {
        CompletionResponse {
            text: std::borrow::Cow::Borrowed(""),
            model: std::borrow::Cow::Borrowed("unknown"),
            provider: None,
            usage: None,
            finish_reason: None,
            response_time_ms: None,
            generation_time_ms: None,
            tokens_per_second: None,
        }
    }
}

impl MessageChunk for CompletionResponse<'_> {
    fn bad_chunk(error: String) -> Self {
        CompletionResponse {
            text: std::borrow::Cow::Owned(format!("Error: {error}")),
            model: std::borrow::Cow::Borrowed("error"),
            provider: Some(std::borrow::Cow::Borrowed("error")),
            usage: None,
            finish_reason: Some(std::borrow::Cow::Borrowed("error")),
            response_time_ms: None,
            generation_time_ms: None,
            tokens_per_second: None,
        }
    }

    fn error(&self) -> Option<&str> {
        if let Some(reason) = &self.finish_reason {
            if reason == "error" {
                Some(&self.text)
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// A more compact representation of a completion response using Arcs for shared ownership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactCompletionResponse {
    /// The generated completion text
    pub content: String,
    /// The model that generated the completion
    pub model: String,
    /// The provider that generated the completion
    pub provider: String,
    /// Total tokens used in the completion
    pub tokens_used: u32,
    /// The reason the completion finished
    pub finish_reason: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

impl CompactCompletionResponse {
    /// Convert back to a standard `CompletionResponse`
    #[must_use]
    pub fn into_standard(self) -> CompletionResponse<'static> {
        CompletionResponse {
            text: Cow::Owned((*self.content).to_owned()),
            model: Cow::Owned((*self.model).to_owned()),
            provider: Some(Cow::Owned((*self.provider).to_owned())),
            usage: Some(CandleUsage {
                total_tokens: self.tokens_used,
                input_tokens: 0,  // Not available in compact form
                output_tokens: 0, // Not available in compact form
            }),
            finish_reason: Some(Cow::Owned((*self.finish_reason).to_owned())),
            response_time_ms: if self.response_time_ms > 0 {
                Some(self.response_time_ms)
            } else {
                None
            },
            generation_time_ms: None, // Not available in compact form
            tokens_per_second: None,  // Not available in compact form
        }
    }
}

impl Default for CompactCompletionResponse {
    fn default() -> Self {
        CompactCompletionResponse {
            content: String::new(),
            model: "unknown".to_string(),
            provider: "unknown".to_string(),
            tokens_used: 0,
            finish_reason: "stop".to_string(),
            response_time_ms: 0,
        }
    }
}

impl MessageChunk for CompactCompletionResponse {
    fn bad_chunk(error: String) -> Self {
        CompactCompletionResponse {
            content: format!("Error: {error}"),
            model: "error".to_string(),
            provider: "error".to_string(),
            tokens_used: 0,
            finish_reason: "error".to_string(),
            response_time_ms: 0,
        }
    }

    fn error(&self) -> Option<&str> {
        if self.finish_reason.as_str() == "error" {
            Some(&self.content)
        } else {
            None
        }
    }
}
