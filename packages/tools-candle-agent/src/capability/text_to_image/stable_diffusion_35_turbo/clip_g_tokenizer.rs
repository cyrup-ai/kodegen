//! CLIP-G Tokenizer Download Helper
//!
//! Provides `CandleModel` implementation for downloading tokenizer files from
//! the LAION CLIP-ViT-bigG-14 repository on HuggingFace.
//!
//! This is not a full model implementation - it exists solely to enable
//! architecture-compliant file downloads using `CandleModel.huggingface_file()`.
//!
//! # Usage
//!
//! ```no_run
//! use kodegen_candle_agent::capability::text_to_image::clip_g_tokenizer::ClipGTokenizer;
//! use kodegen_candle_agent::domain::model::traits::CandleModel;
//!
//! let tokenizer_path = ClipGTokenizer.huggingface_file("tokenizer.json")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::domain::model::{CandleModel, CandleModelInfo};

/// CLIP-G tokenizer download helper
///
/// Zero-allocation struct that implements `CandleModel` to enable downloading
/// tokenizer files from `laion/CLIP-ViT-bigG-14-laion2B-39B-b160k` via `huggingface_file()`.
///
/// Used by Stable Diffusion 3.5 for text encoding.
#[derive(Debug, Clone, Copy)]
pub struct ClipGTokenizer;

impl Default for ClipGTokenizer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ClipGTokenizer {
    /// Create new CLIP-G tokenizer helper
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// Static model info for CLIP-G tokenizer
static CLIP_G_TOKENIZER_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::LAION,
    name: "clip-vit-bigg-14-laion-tokenizer",
    registry_key: "laion/CLIP-ViT-bigG-14-laion2B-39B-b160k",
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
    model_id: "clip-g-tok",
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

impl CandleModel for ClipGTokenizer {
    #[inline]
    fn info(&self) -> &'static CandleModelInfo {
        &CLIP_G_TOKENIZER_INFO
    }
}
