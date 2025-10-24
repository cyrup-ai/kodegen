//! T5 Tokenizer Download Helper
//!
//! Provides `CandleModel` implementation for downloading tokenizer files from
//! the community-maintained MT5 tokenizers repository on HuggingFace.
//!
//! This is not a full model implementation - it exists solely to enable
//! architecture-compliant file downloads using `CandleModel.huggingface_file()`.
//!
//! # Usage
//!
//! ```no_run
//! use kodegen_candle_agent::capability::text_to_image::t5_tokenizer::T5TokenizerModel;
//! use kodegen_candle_agent::domain::model::traits::CandleModel;
//!
//! let tokenizer_path = T5TokenizerModel.huggingface_file("t5-v1_1-xxl.tokenizer.json")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::domain::model::{CandleModel, CandleModelInfo};

/// T5 tokenizer download helper
///
/// Zero-allocation struct that implements `CandleModel` to enable downloading
/// tokenizer files from `lmz/mt5-tokenizers` via `huggingface_file()`.
///
/// Used by Stable Diffusion 3.5 for T5-XXL text encoding.
#[derive(Debug, Clone, Copy)]
pub struct T5TokenizerModel;

impl Default for T5TokenizerModel {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl T5TokenizerModel {
    /// Create new T5 tokenizer helper
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// Static model info for T5 tokenizer
static T5_TOKENIZER_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::Community,
    name: "mt5-xxl-tokenizer",
    registry_key: "lmz/mt5-tokenizers",
    quantization_url: None,
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
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "t5-tok",
    quantization: "none",
    patch: None,
    embedding_dimension: None,
    vocab_size: None,
    image_size: None,
    image_mean: None,
    image_std: None,
    default_temperature: None,
    default_top_k: None,
    default_top_p: None,
    supports_kv_cache: false,
    supports_flash_attention: false,
    use_bf16: false,
    default_steps: None,
    default_guidance_scale: None,
    time_shift: None,
    est_memory_allocation_mb: 0,
};

impl CandleModel for T5TokenizerModel {
    #[inline]
    fn info(&self) -> &'static CandleModelInfo {
        &T5_TOKENIZER_INFO
    }
}
