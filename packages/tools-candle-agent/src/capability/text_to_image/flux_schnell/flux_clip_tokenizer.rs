//! CLIP Tokenizer Download Helper for FLUX
//!
//! Provides `CandleModel` implementation for downloading tokenizer and model files from
//! the OpenAI CLIP repository.
//!
//! This enables architecture-compliant file downloads using `CandleModel.huggingface_file()`.

use crate::domain::model::traits::CandleModel;
use crate::domain::model::{CandleModelInfo, CandleProvider};

/// CLIP tokenizer and model download helper for FLUX
///
/// Zero-allocation struct that implements `CandleModel` to enable downloading
/// both tokenizer.json and model.safetensors from the OpenAI CLIP repository via `huggingface_file()`.
#[derive(Debug, Clone, Copy)]
pub struct FluxClipTokenizer;

static CLIP_TOKENIZER_INFO: CandleModelInfo = CandleModelInfo {
    provider: CandleProvider::OpenAI,
    name: "clip-vit-large-patch14",
    registry_key: "openai/clip-vit-large-patch14",
    quantization_url: None,
    max_input_tokens: None,
    max_output_tokens: None,
    input_price: None,
    output_price: None,
    supports_vision: true,
    supports_function_calling: false,
    supports_streaming: false,
    supports_embeddings: false,
    requires_max_tokens: false,
    supports_thinking: false,
    optimal_thinking_budget: None,
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "clip-tokenizer",
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

impl CandleModel for FluxClipTokenizer {
    fn info(&self) -> &'static CandleModelInfo {
        &CLIP_TOKENIZER_INFO
    }
}
