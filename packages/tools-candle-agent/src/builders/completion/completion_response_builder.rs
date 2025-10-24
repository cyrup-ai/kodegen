use std::borrow::Cow;
use std::pin::Pin;
use std::sync::Arc;

use tokio_stream::Stream;

use crate::domain::{
    completion::{
        CandleCompactCompletionResponse as CompactCompletionResponse,
        CandleCompletionResponse as CompletionResponse,
    },
    model::CandleUsage as Usage,
};

/// Builder for completion responses
pub struct CompletionResponseBuilder<'a> {
    inner: CompletionResponse<'a>,
}

impl<'a> CompletionResponseBuilder<'a> {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            inner: CompletionResponse {
                text: Cow::Borrowed(""),
                model: Cow::Borrowed(""),
                provider: None,
                usage: None,
                finish_reason: None,
                response_time_ms: None,
                generation_time_ms: None,
                tokens_per_second: None,
            },
        }
    }

    /// Set the completion text
    pub fn text<S: Into<Cow<'a, str>>>(mut self, text: S) -> Self {
        self.inner.text = text.into();
        self
    }

    /// Set the model name
    pub fn model<S: Into<Cow<'a, str>>>(mut self, model: S) -> Self {
        self.inner.model = model.into();
        self
    }

    /// Set the provider name
    pub fn provider<S: Into<Cow<'a, str>>>(mut self, provider: S) -> Self {
        self.inner.provider = Some(provider.into());
        self
    }

    /// Set the token usage
    pub fn usage(mut self, usage: Usage) -> Self {
        self.inner.usage = Some(usage);
        self
    }

    /// Set the finish reason
    pub fn finish_reason<S: Into<Cow<'a, str>>>(mut self, reason: S) -> Self {
        self.inner.finish_reason = Some(reason.into());
        self
    }

    /// Set the response time in milliseconds
    pub fn response_time_ms(mut self, ms: u64) -> Self {
        self.inner.response_time_ms = Some(ms);
        self
    }

    /// Set the number of tokens generated (output tokens)
    pub fn tokens_generated(mut self, tokens: u32) -> Self {
        let usage = self.inner.usage.get_or_insert_with(Usage::zero);
        usage.output_tokens = tokens;
        usage.total_tokens = usage.input_tokens + tokens;
        self
    }

    /// Build the completion response
    pub fn build(self) -> CompletionResponse<'a> {
        self.inner
    }
}

impl Default for CompletionResponseBuilder<'static> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for `CompactCompletionResponse`
pub struct CompactCompletionResponseBuilder {
    content: Option<Arc<str>>,
    model: Option<Arc<str>>,
    provider: Option<Arc<str>>,
    tokens_used: u32,
    finish_reason: Option<Arc<str>>,
    response_time_ms: u64,
}

impl CompactCompletionResponseBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            content: None,
            model: None,
            provider: None,
            tokens_used: 0,
            finish_reason: None,
            response_time_ms: 0,
        }
    }

    /// Set the content
    pub fn content(mut self, content: impl Into<Arc<str>>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<Arc<str>>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the provider
    pub fn provider(mut self, provider: impl Into<Arc<str>>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set tokens used
    pub fn tokens_used(mut self, tokens: u32) -> Self {
        self.tokens_used = tokens;
        self
    }

    /// Set finish reason
    pub fn finish_reason(mut self, reason: impl Into<Arc<str>>) -> Self {
        self.finish_reason = Some(reason.into());
        self
    }

    /// Set response time
    pub fn response_time_ms(mut self, ms: u64) -> Self {
        self.response_time_ms = ms;
        self
    }

    /// Build the compact response
    pub fn build(self) -> Pin<Box<dyn Stream<Item = CompactCompletionResponse> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let response = CompactCompletionResponse {
                    content: self.content.map(|s| s.to_string()).unwrap_or_default(),
                    model: self
                        .model
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    provider: self
                        .provider
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    tokens_used: self.tokens_used,
                    finish_reason: self
                        .finish_reason
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "stop".to_string()),
                    response_time_ms: self.response_time_ms,
                };

                let _ = sender.send(response);
            },
        ))
    }
}

impl Default for CompactCompletionResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
