//! Tests for autocompletion with fuzzy matching

use kodegen_candle_agent::cli::completion::*;

#[test]
fn test_model_completer() {
    let completer = ModelCompleter::new();
    let matches = completer.complete("phi");
    assert!(!matches.is_empty());
    assert!(
        matches
            .iter()
            .any(|m| m.contains("phi") || m.contains("Phi"))
    );
}

#[test]
fn test_model_best_match() {
    let completer = ModelCompleter::new();
    let best = completer.best_match("phi4");
    assert!(best.is_some());
}

#[test]
fn test_command_completer() {
    let completer = CommandCompleter::new();
    let matches = completer.complete("/he");
    assert!(matches.contains(&"/help".to_string()));
}

#[test]
fn test_parse_command() {
    let result = CommandCompleter::parse_command("/save myfile.txt");
    assert!(result.is_some());
    let (cmd, args) = result.unwrap();
    assert_eq!(cmd, "/save");
    assert_eq!(args, vec!["myfile.txt"]);
}

#[test]
fn test_is_command() {
    assert!(CommandCompleter::is_command("/help"));
    assert!(!CommandCompleter::is_command("hello"));
}
