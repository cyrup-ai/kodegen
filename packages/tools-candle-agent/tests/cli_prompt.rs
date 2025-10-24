//! Tests for interactive prompt builder

use kodegen_candle_agent::cli::prompt::*;

#[test]
fn test_prompt_builder_creation() {
    let builder = PromptBuilder::new();
    // Builder creation succeeds - this validates ModelCompleter initialization
    drop(builder);
}

#[test]
fn test_suggest_commands() {
    let builder = PromptBuilder::new();
    let suggestions = builder.suggest_commands("/he");
    assert!(suggestions.contains(&"/help".to_string()));
}

#[test]
fn test_non_command_suggestions() {
    let builder = PromptBuilder::new();
    let suggestions = builder.suggest_commands("hello");
    assert!(suggestions.is_empty());
}
