//! Unit tests for message parser
//!
//! Tests the parsing of JSON values into typed Message objects

use kodegen_tools_claude_agent::parse_message;
use serde_json::json;

#[test]
fn test_parse_user_message() {
    let data = json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": "Hello, Claude!"
        }
    });

    let result = parse_message(data);
    assert!(result.is_ok());
}

#[test]
fn test_parse_invalid_message() {
    let data = json!({
        "type": "invalid_type",
        "data": "some data"
    });

    let result = parse_message(data);
    assert!(result.is_err());
}
