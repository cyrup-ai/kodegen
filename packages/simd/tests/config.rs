use kodegen_simd::config::{ConfigError, ProcessorConfig};

#[test]
fn test_default_config() {
    let config = ProcessorConfig::default();
    assert_eq!(config.temperature, 1.0);
    assert_eq!(config.top_k, None);
    assert_eq!(config.top_p, None);
    assert_eq!(config.repetition_penalty, 1.0);
    assert_eq!(config.frequency_penalty, 0.0);
    assert_eq!(config.presence_penalty, 0.0);
}

#[test]
fn test_validation() {
    let mut config = ProcessorConfig::default()
        .with_temperature(0.5)
        .with_top_p(Some(0.9))
        .with_repetition_penalty(1.2)
        .with_frequency_penalty(0.1)
        .with_presence_penalty(0.1);

    assert!(config.validate().is_ok());

    // Test invalid temperature
    config.temperature = -1.0;
    assert!(matches!(
        config.validate(),
        Err(ConfigError::InvalidTemperature(_))
    ));
    config.temperature = 1.0;

    // Test invalid top_p
    config.top_p = Some(1.1);
    assert!(matches!(
        config.validate(),
        Err(ConfigError::InvalidTopP(_))
    ));
    config.top_p = Some(0.9);

    // Test invalid repetition penalty
    config.repetition_penalty = 0.5;
    assert!(matches!(
        config.validate(),
        Err(ConfigError::InvalidRepetitionPenalty(_))
    ));
    config.repetition_penalty = 1.2;

    // Test invalid frequency penalty
    config.frequency_penalty = -0.1;
    assert!(matches!(
        config.validate(),
        Err(ConfigError::InvalidFrequencyPenalty(_))
    ));
    config.frequency_penalty = 0.1;

    // Test invalid presence penalty
    config.presence_penalty = -0.1;
    assert!(matches!(
        config.validate(),
        Err(ConfigError::InvalidPresencePenalty(_))
    ));
}
