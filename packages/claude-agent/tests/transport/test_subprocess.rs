//! Unit tests for `SubprocessTransport`
//!
//! Tests the subprocess transport implementation

use kodegen_claude_agent::transport::{PromptInput, SubprocessTransport};

#[test]
fn test_find_cli() {
    // This will succeed if claude is installed
    let result = SubprocessTransport::find_cli();
    // We can't assert success because it depends on environment
    println!("CLI search result: {result:?}");
}

#[test]
fn test_prompt_input_conversions() {
    let _prompt1: PromptInput = "hello".into();
    let _prompt2: PromptInput = String::from("world").into();
}
