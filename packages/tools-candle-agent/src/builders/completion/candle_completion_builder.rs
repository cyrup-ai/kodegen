use arrayvec::ArrayVec;
use smallvec::SmallVec;

use crate::domain::completion::{
    CompletionCoreError, CompletionCoreRequest, CompletionCoreResponse, CompletionCoreResult,
    candle::{MAX_PROMPT_SIZE, MAX_RESPONSE_SIZE, MAX_STOP_TOKENS, MAX_TOKEN_BUFFER},
    types::CandleModelParams as ModelParams,
};

/// Zero-allocation completion request builder for high-performance use cases
pub struct CompletionCoreRequestBuilder<'a> {
    prompt: ArrayVec<u8, MAX_PROMPT_SIZE>,
    max_tokens: u32,
    temperature: f32,
    top_k: u32,
    top_p: f32,
    stop_tokens: SmallVec<&'a str, MAX_STOP_TOKENS>,
    stream: bool,
    model_params: ModelParams,
    seed: Option<u64>,
}

impl<'a> CompletionCoreRequestBuilder<'a> {
    /// Create a new builder
    #[inline]
    pub fn new() -> Self {
        Self {
            prompt: ArrayVec::new(),
            max_tokens: 100,
            temperature: 1.0,
            top_k: 50,
            top_p: 0.9,
            stop_tokens: SmallVec::new(),
            stream: false,
            model_params: ModelParams::default(),
            seed: None,
        }
    }

    /// Set the prompt text
    #[inline]
    pub fn prompt<S: AsRef<str>>(mut self, prompt: S) -> Self {
        let prompt_bytes = prompt.as_ref().as_bytes();
        self.prompt.clear();
        self.prompt.try_extend_from_slice(prompt_bytes).ok();
        self
    }

    /// Set maximum tokens to generate
    #[inline]
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens.min(MAX_TOKEN_BUFFER as u32);
        self
    }

    /// Set sampling temperature
    #[inline]
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set top-k sampling parameter
    #[inline]
    pub fn top_k(mut self, top_k: u32) -> Self {
        self.top_k = top_k;
        self
    }

    /// Set top-p sampling parameter
    #[inline]
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p.clamp(0.0, 1.0);
        self
    }

    /// Add a stop token
    #[inline]
    pub fn stop_token(mut self, token: &'a str) -> Self {
        if self.stop_tokens.len() < MAX_STOP_TOKENS {
            self.stop_tokens.push(token);
        }
        self
    }

    /// Enable streaming response
    #[inline]
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Set model parameters
    #[inline]
    pub fn model_params(mut self, params: ModelParams) -> Self {
        self.model_params = params;
        self
    }

    /// Set random seed
    #[inline]
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Build the completion request
    #[inline]
    pub fn build(self) -> CompletionCoreResult<CompletionCoreRequest<'a>> {
        if self.prompt.is_empty() {
            return Err(CompletionCoreError::InvalidRequest(String::from(
                "prompt cannot be empty",
            )));
        }

        if self.max_tokens == 0 {
            return Err(CompletionCoreError::InvalidRequest(String::from(
                "max_tokens must be > 0",
            )));
        }

        Ok(CompletionCoreRequest::from_builder(
            self.prompt,
            self.max_tokens,
            self.temperature.into(), // f32 -> f64
            self.top_k,
            self.top_p.into(), // f32 -> f64
            self.stop_tokens,
            self.stream,
            self.model_params,
            self.seed,
        ))
    }
}

impl Default for CompletionCoreRequestBuilder<'static> {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero-allocation completion response builder
pub struct CompletionCoreResponseBuilder {
    text: ArrayVec<u8, MAX_RESPONSE_SIZE>,
    tokens_generated: u32,
    generation_time_ms: u32,
    tokens_per_second: u32,
    finish_reason: ArrayVec<u8, 32>,
    model: ArrayVec<u8, 64>,
}

impl CompletionCoreResponseBuilder {
    /// Create a new builder
    #[inline]
    pub fn new() -> Self {
        Self {
            text: ArrayVec::new(),
            tokens_generated: 0,
            generation_time_ms: 0,
            tokens_per_second: 0,
            finish_reason: ArrayVec::new(),
            model: ArrayVec::new(),
        }
    }

    /// Set the generated text
    #[inline]
    pub fn text<S: AsRef<str>>(mut self, text: S) -> Self {
        let text_bytes = text.as_ref().as_bytes();
        self.text.clear();
        self.text.try_extend_from_slice(text_bytes).ok();
        self
    }

    /// Set tokens generated
    #[inline]
    pub fn tokens_generated(mut self, tokens: u32) -> Self {
        self.tokens_generated = tokens;
        self
    }

    /// Set generation time
    #[inline]
    pub fn generation_time_ms(mut self, time_ms: u32) -> Self {
        self.generation_time_ms = time_ms;
        self
    }

    /// Set tokens per second
    #[inline]
    pub fn tokens_per_second(mut self, tps: u32) -> Self {
        self.tokens_per_second = tps;
        self
    }

    /// Set finish reason
    #[inline]
    pub fn finish_reason<S: AsRef<str>>(mut self, reason: S) -> Self {
        let reason_bytes = reason.as_ref().as_bytes();
        self.finish_reason.clear();
        self.finish_reason.try_extend_from_slice(reason_bytes).ok();
        self
    }

    /// Set model name
    #[inline]
    pub fn model<S: AsRef<str>>(mut self, model: S) -> Self {
        let model_bytes = model.as_ref().as_bytes();
        self.model.clear();
        self.model.try_extend_from_slice(model_bytes).ok();
        self
    }

    /// Build the completion response
    #[inline]
    pub fn build(self) -> CompletionCoreResult<CompletionCoreResponse> {
        if self.text.is_empty() {
            return Err(CompletionCoreError::Internal(String::from(
                "response text cannot be empty",
            )));
        }

        Ok(CompletionCoreResponse::from_builder(
            self.text,
            self.tokens_generated,
            self.generation_time_ms,
            self.tokens_per_second,
            self.finish_reason,
            self.model,
        ))
    }
}

impl Default for CompletionCoreResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
