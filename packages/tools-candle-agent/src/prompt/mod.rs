use serde::{Deserialize, Serialize};

use crate::domain::chat::message::types::CandleMessageRole as MessageRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub content: String,
    #[serde(default = "default_role")]
    pub role: MessageRole,
}

fn default_role() -> MessageRole {
    MessageRole::User
}

impl Prompt {
    pub fn new(content: impl Into<String>) -> Self {
        Prompt {
            content: content.into(),
            role: MessageRole::User,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

// PromptBuilder moved to cyrup/src/builders/prompt.rs

impl From<Prompt> for String {
    fn from(val: Prompt) -> Self {
        val.content
    }
}

// PromptBuilder implementation moved to cyrup/src/builders/prompt.rs

/// Candle-prefixed type alias for domain compatibility
pub type CandlePrompt = Prompt;
