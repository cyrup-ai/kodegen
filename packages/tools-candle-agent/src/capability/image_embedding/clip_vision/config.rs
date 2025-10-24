//! CLIP Vision model configuration and metadata
//!
//! This module contains static configuration data for CLIP Vision models
//! and helper functions for building CLIP configs based on model dimensions.

use crate::domain::model::CandleModelInfo;
use candle_transformers::models::clip::text_model::ClipTextConfig;
use candle_transformers::models::clip::vision_model::ClipVisionConfig;

/// Get CLIP configs for the specified dimension
///
/// Returns (text_config, vision_config, image_size) tuple.
/// Note: text_config is required by ClipModel but unused for vision-only inference.
pub fn get_configs_for_dimension(dimension: usize) -> (ClipTextConfig, ClipVisionConfig, usize) {
    use candle_transformers::models::clip::text_model::Activation;

    match dimension {
        512 => (
            ClipTextConfig::vit_base_patch32(),
            ClipVisionConfig::vit_base_patch32(),
            224, // image_size for Base
        ),
        768 => (
            // Manual ClipTextConfig for Large (unused in vision-only inference)
            ClipTextConfig {
                vocab_size: 49408,
                embed_dim: 768,
                intermediate_size: 3072,
                max_position_embeddings: 77,
                pad_with: None,
                num_hidden_layers: 12,
                num_attention_heads: 12,
                projection_dim: 768,
                activation: Activation::QuickGelu,
            },
            ClipVisionConfig::clip_vit_large_patch14_336(),
            336, // image_size for Large
        ),
        _ => unreachable!("Dimension validated in constructor"),
    }
}

/// Static model info for CLIP Vision Base (512D)
pub static CLIP_VISION_BASE_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::OpenAI,
    name: "clip-vit-base-patch32",
    registry_key: "openai/clip-vit-base-patch32",
    quantization_url: None,
    max_input_tokens: None,
    max_output_tokens: None,
    input_price: None,
    output_price: None,
    supports_vision: true,
    supports_function_calling: false,
    supports_streaming: false,
    supports_embeddings: true,
    requires_max_tokens: false,
    supports_thinking: false,
    optimal_thinking_budget: None,
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "clip-vision-base",
    quantization: "none",
    patch: None,
    embedding_dimension: Some(512),
    vocab_size: None,
    image_size: Some(224),
    image_mean: Some([0.48145466, 0.4578275, 0.40821073]),
    image_std: Some([0.26862954, 0.261_302_6, 0.275_777_1]),
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

/// Static model info for CLIP Vision Large (768D)  
pub static CLIP_VISION_LARGE_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::OpenAI,
    name: "clip-vit-large-patch14-336",
    registry_key: "openai/clip-vit-large-patch14-336",
    quantization_url: None,
    max_input_tokens: None,
    max_output_tokens: None,
    input_price: None,
    output_price: None,
    supports_vision: true,
    supports_function_calling: false,
    supports_streaming: false,
    supports_embeddings: true,
    requires_max_tokens: false,
    supports_thinking: false,
    optimal_thinking_budget: None,
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "clip-vision-large",
    quantization: "none",
    patch: None,
    embedding_dimension: Some(768),
    vocab_size: None,
    image_size: Some(336),
    image_mean: Some([0.48145466, 0.4578275, 0.40821073]),
    image_std: Some([0.26862954, 0.261_302_6, 0.275_777_1]),
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
