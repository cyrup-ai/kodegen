// Tests extracted from src/cli/handler.rs

use kodegen_candle_agent::cli::{InputHandler, InputHandlerResult, CommandResult, CliConfig};

#[test]
fn test_handle_regular_message() {
    let mut handler = InputHandler::new(CliConfig::new());
    let result = handler.handle("Hello, world!");

    match result {
        InputHandlerResult::Chat(msg) => assert_eq!(msg, "Hello, world!"),
        _ => panic!("Expected Chat result"),
    }
}

#[test]
fn test_handle_exit_command() {
    let mut handler = InputHandler::new(CliConfig::new());
    let result = handler.handle("/exit");

    matches!(result, InputHandlerResult::Exit);
}

#[test]
fn test_handle_help_command() {
    let mut handler = InputHandler::new(CliConfig::new());
    let result = handler.handle("/help");

    match result {
        InputHandlerResult::Command(CommandResult::Help(text)) => {
            assert!(text.contains("Available Commands"));
        }
        _ => panic!("Expected Help result"),
    }
}

#[test]
fn test_handle_temperature_command() {
    let mut handler = InputHandler::new(CliConfig::new());
    let result = handler.handle("/temperature 0.5");

    match result {
        InputHandlerResult::Command(CommandResult::ConfigChanged(_)) => {
            assert_eq!(handler.config().default_temperature, 0.5);
        }
        _ => panic!("Expected ConfigChanged result"),
    }
}
