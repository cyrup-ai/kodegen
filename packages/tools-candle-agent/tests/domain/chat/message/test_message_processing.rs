// Tests extracted from src/domain/chat/message/message_processing.rs

use kodegen_candle_agent::domain::chat::message::{CandleMessage, CandleMessageRole};
use kodegen_candle_agent::domain::chat::message::message_processing::{
    process_message, validate_message, sanitize_content,
};
use tokio_stream::StreamExt;

#[tokio::test]
async fn test_process_message() {
    let message = CandleMessage {
        role: CandleMessageRole::User,
        content: "  Hello, world!  ".to_string(),
        id: None,
        timestamp: None,
    };

    let processed: Vec<_> = process_message(message).collect().await;
    assert_eq!(processed.len(), 1);
    assert_eq!(processed[0].content, "Hello, world!");
}

#[tokio::test]
async fn test_validate_message() {
    let valid_message = CandleMessage {
        role: CandleMessageRole::User,
        content: "Hello, world!".to_string(),
        id: None,
        timestamp: None,
    };

    let empty_message = CandleMessage {
        role: CandleMessageRole::User,
        content: "   ".to_string(),
        id: None,
        timestamp: None,
    };

    let valid_stream = validate_message(valid_message);
    let valid_results: Vec<CandleMessage> = valid_stream.collect().await;
    assert_eq!(valid_results[0].content, "Hello, world!");

    let empty_stream = validate_message(empty_message);
    let empty_results: Vec<CandleMessage> = empty_stream.collect().await;
    assert_eq!(empty_results[0].content, "   "); // Validation is now handled by on_chunk handler
}

#[test]
fn test_sanitize_content() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(sanitize_content("  Hello, world!  ")?, "Hello, world!");
    assert_eq!(sanitize_content("")?, "");
    assert_eq!(sanitize_content("  ")?, "");
    Ok(())
}
