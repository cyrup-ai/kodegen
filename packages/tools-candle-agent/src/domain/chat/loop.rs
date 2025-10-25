//! Chat loop control flow types for the unified domain chat system.

use std::fmt;

/// Controls the flow of a chat conversation in the unified domain system.
///
/// This enum is used to control the flow of a chat conversation, allowing for breaking out of loops,
/// sending responses, or prompting the user for more input. This is the unified version that combines
/// the best features from both the original chat system and builder patterns.
#[derive(Debug, Clone, PartialEq)]
pub enum CandleChatLoop {
    /// Break out of the chat loop and end the conversation.
    Break,

    /// Send a response back to the user and continue the conversation.
    /// The String contains the response message.
    Reprompt(String),

    /// Prompt the user for input and continue the conversation.
    /// The String contains the prompt message.
    UserPrompt(String),
}

impl fmt::Display for CandleChatLoop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CandleChatLoop::Break => write!(f, "CandleChatLoop::Break"),
            CandleChatLoop::Reprompt(msg) => write!(f, "CandleChatLoop::Reprompt({msg:?})"),
            CandleChatLoop::UserPrompt(prompt) => {
                write!(f, "CandleChatLoop::UserPrompt({prompt:?})")
            }
        }
    }
}
