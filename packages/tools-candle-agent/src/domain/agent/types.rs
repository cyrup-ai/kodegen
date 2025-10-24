//! Additional agent-related types and utilities

use std::collections::HashMap;

use serde_json::Value;

use crate::domain::chat::message::types::CandleMessageRole as MessageRole;
use crate::domain::context::CandleContext;
use cyrup_sugars::ZeroOneOrMany;
// Type aliases for trait objects
type Context = Box<dyn CandleContext + Send + Sync>;
use uuid;

/// Additional parameters for agent configuration
/// A simple key-value store for provider-specific parameters
#[derive(Debug, Clone, Default)]
pub struct CandleAdditionalParams {
    /// Internal storage for parameters
    params: HashMap<String, Value>,
}

impl CandleAdditionalParams {
    /// Create a new empty set of additional parameters
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    /// Create from a map of parameters
    #[must_use]
    pub fn from_map(params: HashMap<String, Value>) -> Self {
        Self { params }
    }

    /// Get a parameter value by key
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.params.get(key)
    }

    /// Set a parameter value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.params.insert(key.into(), value.into());
    }

    /// Check if a parameter exists
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }

    /// Get a reference to the underlying parameters
    #[must_use]
    pub fn as_map(&self) -> &HashMap<String, Value> {
        &self.params
    }
}

/// Agent helper type for conversation turn callbacks.
/// Note: This is defined here for legacy compatibility.
/// See `CandleAgentRoleAgent` in `domain/agent/role` for the active implementation.
pub struct AgentRoleAgent;

/// Main agent type for the Candle framework
#[derive(Debug, Clone)]
pub struct CandleAgent {
    /// Agent identifier
    pub id: String,
    /// Agent configuration
    pub config: AgentConfig,
    /// Current conversation state
    pub conversation: AgentConversation,
}

/// Agent configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent name/role
    pub name: String,
    /// System prompt template
    pub system_prompt: Option<String>,
    /// Temperature for generation
    pub temperature: f64,
    /// Maximum tokens
    pub max_tokens: Option<usize>,
}

impl CandleAgent {
    /// Create a new agent with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            config: AgentConfig {
                name: name.into(),
                system_prompt: None,
                temperature: 0.0, // Greedy sampling for agents - deterministic output
                max_tokens: None,
            },
            conversation: AgentConversation::new(),
        }
    }

    /// Get the agent's name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Set the system prompt
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.system_prompt = Some(prompt.into());
        self
    }

    /// Set the temperature
    #[must_use]
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.config.temperature = temperature;
        self
    }

    /// Set max tokens
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.config.max_tokens = Some(max_tokens);
        self
    }
}

/// Agent conversation type
#[derive(Debug, Clone)]
pub struct AgentConversation {
    /// Optional collection of conversation messages with their roles
    pub messages: Option<ZeroOneOrMany<(MessageRole, String)>>,
}

impl AgentConversation {
    /// Create a new empty agent conversation
    #[must_use]
    pub fn new() -> Self {
        Self { messages: None }
    }

    /// Get the last message from the conversation
    #[must_use]
    pub fn last(&self) -> AgentConversationMessage {
        AgentConversationMessage {
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
}

impl Default for AgentConversation {
    fn default() -> Self {
        Self::new()
    }
}

/// A single message in an agent conversation
#[derive(Debug, Clone)]
pub struct AgentConversationMessage {
    content: String,
}

impl AgentConversationMessage {
    /// Get the message content as a string slice
    #[must_use]
    pub fn message(&self) -> &str {
        &self.content
    }
}

/// Trait for context arguments - zero-allocation with static dispatch
pub trait ContextArgs {
    /// Add this context to the collection of contexts
    fn add_to(self, contexts: &mut ZeroOneOrMany<Context>);
}

use rmcp::model::Tool as ToolInfo;

/// Trait for tool arguments - zero-allocation with static dispatch
pub trait ToolArgs {
    /// Add this tool to the collection of tools
    fn add_to(self, tools: &mut ZeroOneOrMany<ToolInfo>);
}

/// Trait for conversation history arguments - moved to cyrup/src/builders/
pub trait ConversationHistoryArgs {
    /// Convert this into conversation history format
    fn into_history(self) -> Option<ZeroOneOrMany<(MessageRole, String)>>;
}
