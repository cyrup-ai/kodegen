//! Candle Agent role trait and implementation - EXACT REPLICA of domain with Candle prefixes

use cyrup_sugars::ZeroOneOrMany;
use std::fmt;

use crate::domain::chat::CandleMessageRole;

/// MCP Server configuration
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    /// MCP server type identification (stdio, socket, etc.)
    server_type: String,
    /// MCP server binary executable path
    bin_path: Option<String>,
    /// MCP server initialization command
    init_command: Option<String>,
}

impl McpServerConfig {
    /// Create a new MCP server configuration
    #[inline]
    #[must_use]
    pub fn new(
        server_type: String,
        bin_path: Option<String>,
        init_command: Option<String>,
    ) -> Self {
        Self {
            server_type,
            bin_path,
            init_command,
        }
    }

    /// Create a stdio-based MCP server configuration
    #[inline]
    pub fn stdio(bin_path: impl Into<String>) -> Self {
        Self {
            server_type: "stdio".to_string(),
            bin_path: Some(bin_path.into()),
            init_command: None,
        }
    }

    /// Create a socket-based MCP server configuration
    #[inline]
    pub fn socket(init_command: impl Into<String>) -> Self {
        Self {
            server_type: "socket".to_string(),
            bin_path: None,
            init_command: Some(init_command.into()),
        }
    }

    /// Get the server type
    #[inline]
    #[must_use]
    pub fn server_type(&self) -> &str {
        &self.server_type
    }

    /// Get the binary path
    #[inline]
    #[must_use]
    pub fn bin_path(&self) -> Option<&str> {
        self.bin_path.as_deref()
    }

    /// Get the initialization command
    #[inline]
    #[must_use]
    pub fn init_command(&self) -> Option<&str> {
        self.init_command.as_deref()
    }
}

/// Core agent role trait defining all operations and properties
pub trait CandleAgentRole: Send + Sync + fmt::Debug + Clone {
    /// Get the name of the agent role
    fn name(&self) -> &str;

    /// Get the temperature setting
    fn temperature(&self) -> f64;

    /// Get the max tokens setting
    fn max_tokens(&self) -> Option<u64>;

    /// Get the memory read timeout in milliseconds
    fn memory_read_timeout(&self) -> Option<u64>;

    /// Get the system prompt
    fn system_prompt(&self) -> Option<&str>;

    /// Create a new agent role with the given name
    fn new(name: impl Into<String>) -> Self;
}

/// Agent helper type provided to `on_conversation_turn` callbacks.
///
/// This type provides the `chat()` method for controlling conversation flow:
/// - `agent.chat(CandleChatLoop::Break)` - Exit the conversation loop
/// - `agent.chat(CandleChatLoop::UserPrompt(msg))` - Send a message
/// - `agent.chat(CandleChatLoop::Reprompt(msg))` - Re-prompt with a message
///
/// # Example
/// ```ignore
/// .on_conversation_turn(|conversation, agent| {
///     if should_exit {
///         agent.chat(CandleChatLoop::Break)
///     } else {
///         agent.chat(CandleChatLoop::UserPrompt("Continue...".to_string()))
///     }
/// })
/// ```
/// Agent conversation type
pub struct CandleAgentConversation {
    /// Conversation messages as role-content pairs
    pub messages: Option<ZeroOneOrMany<(CandleMessageRole, String)>>,
}

impl Default for CandleAgentConversation {
    fn default() -> Self {
        Self::new()
    }
}

impl CandleAgentConversation {
    /// Create a new empty conversation
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self { messages: None }
    }

    /// Create a conversation with an initial user message
    #[inline]
    #[must_use]
    pub fn with_user_input(message: impl Into<String>) -> Self {
        let mut conv = Self::new();
        conv.add_message(message, CandleMessageRole::User);
        conv
    }

    /// Add a message to the conversation
    #[inline]
    pub fn add_message(&mut self, content: impl Into<String>, role: CandleMessageRole) {
        let message = (role, content.into());
        self.messages = Some(match self.messages.take() {
            None => ZeroOneOrMany::One(message),
            Some(existing) => existing.with_pushed(message),
        });
    }

    /// Get the last message from the conversation
    #[inline]
    #[must_use]
    pub fn last(&self) -> CandleAgentConversationMessage {
        CandleAgentConversationMessage {
            content: self
                .messages
                .as_ref()
                .and_then(|msgs| {
                    // Get the last element from ZeroOneOrMany
                    let all: Vec<_> = msgs.clone().into_iter().collect();
                    all.last().map(|(_, m)| m.clone())
                })
                .unwrap_or_default(),
        }
    }

    /// Get the latest user message from the conversation
    #[inline]
    #[must_use]
    pub fn latest_user_message(&self) -> String {
        self.messages
            .as_ref()
            .and_then(|msgs| {
                // Find the last user message
                let all: Vec<_> = msgs.clone().into_iter().collect();
                all.iter()
                    .rev()
                    .find(|(role, _)| matches!(role, CandleMessageRole::User))
                    .map(|(_, content)| content.clone())
            })
            .unwrap_or_default()
    }
}

/// A single message in an agent conversation
pub struct CandleAgentConversationMessage {
    content: String,
}

impl CandleAgentConversationMessage {
    /// Get the message content as a string slice
    #[inline]
    #[must_use]
    pub fn message(&self) -> &str {
        &self.content
    }
}

/// Trait for context arguments - moved to cyrup/src/builders/
pub trait CandleContextArgs {
    /// Add this context to the collection of contexts
    fn add_to(self, contexts: &mut Option<ZeroOneOrMany<Box<dyn std::any::Any + Send + Sync>>>);
}

/// Trait for tool arguments - moved to cyrup/src/builders/
pub trait CandleToolArgs {
    /// Add this tool to the collection of tools
    fn add_to(self, tools: &mut Option<ZeroOneOrMany<Box<dyn std::any::Any + Send + Sync>>>);
}

/// Trait for conversation history arguments - moved to cyrup/src/builders/
pub trait CandleConversationHistoryArgs {
    /// Convert this into conversation history format
    fn into_history(self) -> Option<ZeroOneOrMany<(CandleMessageRole, String)>>;
}

// JSON conversion functions removed - no longer needed with unified serde_json::Value usage
