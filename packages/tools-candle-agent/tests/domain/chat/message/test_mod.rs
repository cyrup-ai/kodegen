// Tests extracted from src/domain/chat/message/mod.rs

use kodegen_candle_agent::domain::chat::message::{
    CandleMessage, CandleMessageRole, processing,
};

#[test]
fn test_candle_message_creation() {
    let message = CandleMessage {
        role: CandleMessageRole::User,
        content: "Hello, world!".to_string(),
        id: Some("123".to_string()),
        timestamp: Some(1_234_567_890),
    };

    assert_eq!(message.role, CandleMessageRole::User);
    assert_eq!(message.content, "Hello, world!");
}

#[test]
fn test_candle_message_processing() -> Result<(), Box<dyn std::error::Error>> {
    let mut message = CandleMessage {
        role: CandleMessageRole::User,
        content: "  Hello, world!  ".to_string(),
        id: None,
        timestamp: None,
    };

    processing::candle_process_message(&mut message)?;
    assert_eq!(message.content, "Hello, world!");
    Ok(())
}
