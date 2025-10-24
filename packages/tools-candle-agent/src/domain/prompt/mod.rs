use serde::{Deserialize, Serialize};

use crate::domain::chat::message::types::CandleMessageRole as MessageRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandlePrompt {
    pub content: String,
    #[serde(default = "default_role")]
    pub role: MessageRole,
}

fn default_role() -> MessageRole {
    MessageRole::User
}

impl CandlePrompt {
    pub fn new(content: impl Into<String>) -> Self {
        CandlePrompt {
            content: content.into(),
            role: MessageRole::User,
        }
    }

    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}

// PromptBuilder moved to cyrup/src/builders/prompt.rs

impl From<CandlePrompt> for String {
    fn from(val: CandlePrompt) -> Self {
        val.content
    }
}

impl std::fmt::Display for CandlePrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}

// PromptBuilder implementation moved to cyrup/src/builders/prompt.rs
