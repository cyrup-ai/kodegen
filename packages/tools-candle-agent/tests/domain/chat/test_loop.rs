// Tests extracted from src/domain/chat/loop.rs

use kodegen_candle_agent::domain::chat::CandleChatLoop;

#[test]
fn test_chat_loop_display() {
    assert_eq!(CandleChatLoop::Break.to_string(), "CandleChatLoop::Break");
    assert_eq!(
        CandleChatLoop::Reprompt("Hello".to_string()).to_string(),
        "CandleChatLoop::Reprompt(\"Hello\")"
    );
    assert_eq!(
        CandleChatLoop::UserPrompt("What's next?".to_string()).to_string(),
        "CandleChatLoop::UserPrompt(\"What's next?\")"
    );
}
