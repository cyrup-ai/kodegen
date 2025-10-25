// Tests extracted from src/core/generation/config.rs

use kodegen_candle_agent::core::generation::config::{
    SamplingConfig, deterministic_config, balanced_config,
};

#[test]
fn test_config_validation() {
    let valid_config = SamplingConfig::new(0.8);
    assert!(valid_config.validate().is_ok());

    let invalid_config = SamplingConfig::new(-1.0);
    assert!(invalid_config.validate().is_err());

    let invalid_top_p = SamplingConfig::new(1.0).with_top_p(1.5);
    assert!(invalid_top_p.validate().is_err());
}

#[test]
fn test_builder_pattern() {
    let config = SamplingConfig::new(0.0)
        .with_top_k(50)
        .with_top_p(0.9)
        .with_repetition_penalty(1.1);

    assert_eq!(config.temperature, 0.0);
    assert_eq!(config.top_k, Some(50));
    assert_eq!(config.top_p, Some(0.9));
    assert_eq!(config.repetition_penalty, 1.1);
}

#[test]
fn test_preset_configs() {
    let det = deterministic_config();
    assert!(det.is_deterministic());
    assert_eq!(det.seed, Some(42));

    let balanced = balanced_config();
    assert!(balanced.validate().is_ok());
    assert!(!balanced.is_deterministic());
}
