//! CLIP-L Tokenizer Download Helper
//!
//! Provides `CandleModel` implementation for downloading tokenizer files from
//! the OpenAI CLIP-ViT-Large-Patch14 repository on HuggingFace.
//!
//! This is not a full model implementation - it exists solely to enable
//! architecture-compliant file downloads using `CandleModel.huggingface_file()`.
//!
//! # Usage
//!
//! ```no_run
//! use kodegen_candle_agent::capability::text_to_image::clip_l_tokenizer::ClipLTokenizer;
//! use kodegen_candle_agent::domain::model::traits::CandleModel;
//!
//! let tokenizer_path = ClipLTokenizer.huggingface_file("tokenizer.json")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::domain::model::{CandleModel, CandleModelInfo};

/// CLIP-L tokenizer download helper
///
/// Zero-allocation struct that implements `CandleModel` to enable downloading
/// tokenizer files from `openai/clip-vit-large-patch14` via `huggingface_file()`.
///
/// Used by Stable Diffusion 3.5 for text encoding.
#[derive(Debug, Clone, Copy)]
pub struct ClipLTokenizer;

impl Default for ClipLTokenizer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ClipLTokenizer {
    /// Create new CLIP-L tokenizer helper
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// Static model info for CLIP-L tokenizer
static CLIP_L_TOKENIZER_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::OpenAI,
    name: "clip-vit-large-patch14-tokenizer",
    registry_key: "openai/clip-vit-large-patch14",
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
    model_id: "clip-l-tok",
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

impl CandleModel for ClipLTokenizer {
    #[inline]
    fn info(&self) -> &'static CandleModelInfo {
        &CLIP_L_TOKENIZER_INFO
    }
}
