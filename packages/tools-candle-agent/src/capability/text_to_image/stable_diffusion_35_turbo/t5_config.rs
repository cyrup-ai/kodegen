//! T5 Config Download Helper
//!
//! Provides `CandleModel` implementation for downloading config files from
//! the Google T5-v1.1-XXL repository on HuggingFace.
//!
//! This is not a full model implementation - it exists solely to enable
//! architecture-compliant file downloads using `CandleModel.huggingface_file()`.
//!
//! # Usage
//!
//! ```no_run
//! use kodegen_candle_agent::capability::text_to_image::t5_config::T5ConfigModel;
//! use kodegen_candle_agent::domain::model::traits::CandleModel;
//!
//! let config_path = T5ConfigModel.huggingface_file("config.json")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::domain::model::{CandleModel, CandleModelInfo};

/// T5 config download helper
///
/// Zero-allocation struct that implements `CandleModel` to enable downloading
/// config files from `google/t5-v1_1-xxl` via `huggingface_file()`.
///
/// Used by Stable Diffusion 3.5 for T5-XXL text encoder configuration.
#[derive(Debug, Clone, Copy)]
pub struct T5ConfigModel;

impl Default for T5ConfigModel {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl T5ConfigModel {
    /// Create new T5 config helper
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// Static model info for T5 config
static T5_CONFIG_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::Google,
    name: "t5-v1_1-xxl-config",
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

impl CandleModel for T5ConfigModel {
    #[inline]
    fn info(&self) -> &'static CandleModelInfo {
        &T5_CONFIG_INFO
    }
}
