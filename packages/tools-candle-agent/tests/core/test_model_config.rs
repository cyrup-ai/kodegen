// Tests extracted from src/core/model_config.rs

use candle_transformers::models::llama::{Config as LlamaConfig, LlamaEosToks};
use kodegen_candle_agent::core::model_config::{
    ModelConfig, ModelArchitecture, SpecialTokenIds,
};
use std::fs::File;
use tempfile::tempdir;

#[test]
fn test_model_config_creation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let model_path = temp_dir.path().join("model.safetensors");
    let tokenizer_path = temp_dir.path().join("tokenizer.json");

    // Create dummy files
    File::create(&model_path)?;
    File::create(&tokenizer_path)?;

    let llama_config = LlamaConfig {
        vocab_size: 32000,
        hidden_size: 4096,
        intermediate_size: 11008,
        num_hidden_layers: 32,
        num_attention_heads: 32,
        num_key_value_heads: 32,
        max_position_embeddings: 2048,
        rms_norm_eps: 1e-6,
        rope_theta: 10000.0,
        use_flash_attn: false,
        bos_token_id: Some(1),
        eos_token_id: Some(LlamaEosToks::Single(2)),
        rope_scaling: None,
        tie_word_embeddings: false,
    };

    let config = ModelConfig::new(
        model_path,
        tokenizer_path,
        ModelArchitecture::Llama(llama_config),
        "test-llama",
        "test-provider",
    );

    assert_eq!(config.registry_key, "test-llama");
    assert_eq!(config.provider_name, "test-provider");
    assert_eq!(config.vocab_size, 32000);

    // Validation should pass with existing files
    config.validate()?;
    Ok(())
}

#[test]
fn test_special_token_identification() {
    let tokens = SpecialTokenIds::default();

    assert!(tokens.is_special_token(1)); // BOS
    assert!(tokens.is_special_token(2)); // EOS
    assert!(tokens.is_special_token(0)); // PAD
    assert!(!tokens.is_special_token(100)); // Regular token

    assert!(tokens.is_eos_token(2));
    assert!(!tokens.is_eos_token(1));

    assert_eq!(tokens.token_name(1), Some("<BOS>"));
    assert_eq!(tokens.token_name(2), Some("<EOS>"));
    assert_eq!(tokens.token_name(100), None);
}

#[test]
fn test_architecture_defaults() {
    let llama_arch = ModelArchitecture::Llama(LlamaConfig {
        vocab_size: 32000,
        hidden_size: 4096,
        intermediate_size: 11008,
        num_hidden_layers: 32,
        num_attention_heads: 32,
        num_key_value_heads: 32,
        max_position_embeddings: 2048,
        rms_norm_eps: 1e-6,
        rope_theta: 10000.0,
        use_flash_attn: false,
        bos_token_id: Some(1),
        eos_token_id: Some(LlamaEosToks::Single(2)),
        rope_scaling: None,
        tie_word_embeddings: false,
    });

    let defaults = llama_arch.get_defaults();
    assert_eq!(defaults.vocab_size, 32000);
    assert_eq!(defaults.context_length, 2048);
    assert_eq!(defaults.special_tokens.eos_token_id, Some(2));

    assert_eq!(llama_arch.name(), "llama");
}
