//! Tests for CLI argument parsing and validation

use kodegen_candle_agent::cli::args::*;

#[test]
fn test_default_args() {
    let args = CliArgs::default();
    assert_eq!(args.agent_role, "CYRUP.ai");
    assert_eq!(args.temperature, 0.0);
    assert_eq!(args.max_tokens, None);
    assert_eq!(args.memory_read_timeout, 5000);
    assert!(args.interactive);
    assert!(args.tools.is_empty());
}

#[test]
fn test_parse_role() {
    let args = vec![
        "program".to_string(),
        "--role".to_string(),
        "MyAgent".to_string(),
    ];
    let cli_args = CliArgs::from_args(&args);
    assert_eq!(cli_args.agent_role, "MyAgent");
}

#[test]
fn test_parse_model() {
    let args = vec![
        "program".to_string(),
        "--model".to_string(),
        "phi4".to_string(),
    ];
    let cli_args = CliArgs::from_args(&args);
    assert_eq!(cli_args.model, Some("phi4".to_string()));
}

#[test]
fn test_parse_temperature() {
    let args = vec!["program".to_string(), "-t".to_string(), "0.5".to_string()];
    let cli_args = CliArgs::from_args(&args);
    assert_eq!(cli_args.temperature, 0.5);
}

#[test]
fn test_parse_memory_timeout() {
    let args = vec![
        "program".to_string(),
        "--memory-read-timeout".to_string(),
        "10000".to_string(),
    ];
    let cli_args = CliArgs::from_args(&args);
    assert_eq!(cli_args.memory_read_timeout, 10000);
}

#[test]
fn test_parse_multiple_tools() {
    let args = vec![
        "program".to_string(),
        "--tool".to_string(),
        "./plugin1.wasm".to_string(),
        "--tool".to_string(),
        "./plugin2.wasm".to_string(),
    ];
    let cli_args = CliArgs::from_args(&args);
    assert_eq!(cli_args.tools.len(), 2);
    assert_eq!(cli_args.tools[0], "./plugin1.wasm");
    assert_eq!(cli_args.tools[1], "./plugin2.wasm");
}

#[test]
fn test_validate_temperature() {
    let mut args = CliArgs {
        temperature: 2.5,
        ..Default::default()
    };
    assert!(args.validate().is_err());

    args.temperature = 0.5;
    assert!(args.validate().is_ok());
}

#[test]
fn test_validate_memory_timeout() {
    let mut args = CliArgs {
        memory_read_timeout: 0,
        ..Default::default()
    };
    assert!(args.validate().is_err());

    args.memory_read_timeout = 5000;
    assert!(args.validate().is_ok());
}
