//! Tests for CLI configuration management

use kodegen_candle_agent::cli::config::*;
use std::env;
use std::fs;

#[test]
fn test_default_config() {
    let config = CliConfig::default();
    assert_eq!(config.default_temperature, 0.0);
    assert_eq!(config.default_max_tokens, 2000);
}

#[test]
fn test_history_management() {
    let mut config = CliConfig::new();
    config.add_to_history("Hello".to_string());
    config.add_to_history("World".to_string());

    assert_eq!(config.get_history().len(), 2);
    assert_eq!(config.get_history()[0], "Hello");

    config.clear_history();
    assert_eq!(config.get_history().len(), 0);
}

#[test]
fn test_save_and_load() {
    let temp_dir = env::temp_dir();
    let config_path = temp_dir.join("test-candle-config.json");

    // Clean up if exists
    let _ = fs::remove_file(&config_path);

    let mut config = CliConfig::new();
    config.set_last_model("phi4".to_string());
    config.add_to_history("test message".to_string());

    // Save
    config.save(Some(&config_path)).unwrap();

    // Load
    let loaded = CliConfig::load(Some(&config_path)).unwrap();
    assert_eq!(loaded.get_last_model(), Some("phi4"));
    assert_eq!(loaded.get_history().len(), 1);

    // Clean up
    let _ = fs::remove_file(&config_path);
}
