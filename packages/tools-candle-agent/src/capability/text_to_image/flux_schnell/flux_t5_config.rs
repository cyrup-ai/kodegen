//! T5 Config Download Helper for FLUX
//!
//! Provides `CandleModel` implementation for downloading config files from
//! the T5 repository.
//!
//! This enables architecture-compliant file downloads using `CandleModel.huggingface_file()`.

use crate::domain::model::traits::CandleModel;
use crate::domain::model::{CandleModelInfo, CandleProvider};

/// T5 config download helper for FLUX
///
/// Zero-allocation struct that implements `CandleModel` to enable downloading
/// config.json from the T5 repository via `huggingface_file()`.
#[derive(Debug, Clone, Copy)]
pub struct FluxT5Config;

static T5_CONFIG_INFO: CandleModelInfo = CandleModelInfo {
    provider: CandleProvider::Google,
    name: "t5-v1_1-xxl",
    registry_key: "google/t5-v1_1-xxl",
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
    model_id: "t5-config",
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

impl CandleModel for FluxT5Config {
    fn info(&self) -> &'static CandleModelInfo {
        &T5_CONFIG_INFO
    }
}
