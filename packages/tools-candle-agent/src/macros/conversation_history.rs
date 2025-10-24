//! Internal macro that enables ARCHITECTURE.md arrow syntax processing

/// Internal implementation detail - processes arrow syntax into tuple format  
/// This macro is used internally by conversation_history method implementations
/// Users never see this - they just use: CandleMessageRole::User => "message"
#[doc(hidden)]
#[macro_export]
macro_rules! __process_conversation_history {
    // Process the arrow syntax and convert to tuples
    ($history:expr) => {{
        // The macro processes whatever ConversationHistoryArgs receives
        // and ensures it gets converted to the right format
        $history.into_history()
    }};
}
