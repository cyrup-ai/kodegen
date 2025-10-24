//! Stella model configuration and helpers

use crate::domain::model::CandleModelInfo;
use candle_transformers::models::stella_en_v5::{EmbedDim, ModelVariant};
use std::num::NonZeroU32;

/// Static model info for Stella 400M variant
pub(crate) static STELLA_400M_MODEL_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::Dunzhang,
    name: "stella_en_400M_v5",
    registry_key: "dunzhang/stella_en_400M_v5",
    quantization_url: None,
    max_input_tokens: NonZeroU32::new(8192),
    max_output_tokens: None,
    input_price: None,
    output_price: None,
    supports_vision: false,
    supports_function_calling: false,
    supports_streaming: false,
    supports_embeddings: true,
    requires_max_tokens: false,
    supports_thinking: false,
    optimal_thinking_budget: None,
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "stella-en-400m-v5",
    quantization: "none",
    patch: None,
    embedding_dimension: Some(1024),
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
    est_memory_allocation_mb: 1600, // 400M params × 4 bytes/param + overhead
};

/// Static model info for Stella 1.5B variant
pub(crate) static STELLA_1_5B_MODEL_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::Dunzhang,
    name: "stella_en_1.5B_v5",
    registry_key: "dunzhang/stella_en_1.5B_v5",
    quantization_url: None,
    max_input_tokens: NonZeroU32::new(8192),
    max_output_tokens: None,
    input_price: None,
    output_price: None,
    supports_vision: false,
    supports_function_calling: false,
    supports_streaming: false,
    supports_embeddings: true,
    requires_max_tokens: false,
    supports_thinking: false,
    optimal_thinking_budget: None,
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "stella-en-1.5b-v5",
    quantization: "none",
    patch: None,
    embedding_dimension: Some(1024),
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
    est_memory_allocation_mb: 7200, // Per official docs: 1.54B params FP32 = 6.17GB + overhead
};

/// Detect model variant from registry_key
pub(crate) fn detect_variant(registry_key: &str) -> ModelVariant {
    if registry_key.contains("1.5B") {
        ModelVariant::Large
    } else {
        ModelVariant::Small // Default to 400M
    }
}

/// Convert dimension to EmbedDim enum
pub(crate) fn embed_dim(
    dimension: u32,
) -> std::result::Result<EmbedDim, Box<dyn std::error::Error + Send + Sync>> {
    match dimension {
        256 => Ok(EmbedDim::Dim256),
        768 => Ok(EmbedDim::Dim768),
        1024 => Ok(EmbedDim::Dim1024),
        2048 => Ok(EmbedDim::Dim2048),
        4096 => Ok(EmbedDim::Dim4096),
        6144 => Ok(EmbedDim::Dim6144),
        8192 => Ok(EmbedDim::Dim8192),
        _ => Err(format!("Unsupported dimension: {}", dimension).into()),
    }
}
