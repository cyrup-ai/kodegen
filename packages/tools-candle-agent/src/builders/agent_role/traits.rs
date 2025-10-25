//! Trait definitions for agent role builders

use super::*;

pub trait CandleAgentRoleBuilder: Sized + Send {
    /// Create a new agent role builder - EXACT syntax: CandleFluentAi::agent_role("name")
    fn new(name: impl Into<String>) -> impl CandleAgentRoleBuilder;

    /// Set text-to-text model - EXACT syntax: .model(registry::get_text_to_text("key").unwrap())
    #[must_use]
    fn model(self, model: TextToTextModel) -> impl CandleAgentRoleBuilder;

    /// Set text embedding model - EXACT syntax: .embedding_model(registry::get_text_embedding("key").unwrap())
    #[must_use]
    fn embedding_model(self, model: TextEmbeddingModel) -> impl CandleAgentRoleBuilder;

    /// Set temperature - EXACT syntax: .temperature(1.0)
    #[must_use]
    fn temperature(self, temp: f64) -> impl CandleAgentRoleBuilder;

    /// Set max tokens - EXACT syntax: .max_tokens(8000)
    #[must_use]
    fn max_tokens(self, max: u64) -> impl CandleAgentRoleBuilder;

    /// Set memory read timeout in milliseconds - EXACT syntax: .memory_read_timeout(5000)
    #[must_use]
    fn memory_read_timeout(self, timeout_ms: u64) -> impl CandleAgentRoleBuilder;

    /// Set system prompt - EXACT syntax: .system_prompt("...")
    #[must_use]
    fn system_prompt(self, prompt: impl Into<String>) -> impl CandleAgentRoleBuilder;

    /// Set additional params - EXACT syntax: .additional_params([("key", "value")])
    #[must_use]
    fn additional_params<P>(self, params: P) -> impl CandleAgentRoleBuilder
    where
        P: IntoIterator<Item = (&'static str, &'static str)>;

    /// Set metadata - EXACT syntax: .metadata([("key", "value")])
    #[must_use]
    fn metadata<Meta>(self, metadata: Meta) -> impl CandleAgentRoleBuilder
    where
        Meta: IntoIterator<Item = (&'static str, &'static str)>;

    /// Set contexts - EXACT syntax: .context(CandleContext::<CandleFile>::of("/path"), CandleContext::<CandleFiles>::glob("*.rs"), ...)
    #[must_use]
    fn context(
        self,
        context1: CandleContext<CandleFile>,
        context2: CandleContext<CandleFiles>,
        context3: CandleContext<CandleDirectory>,
        context4: CandleContext<CandleGithub>,
    ) -> impl CandleAgentRoleBuilder;

    /// Set tools - EXACT syntax: .tools(tool1, tool2, tool3)
    #[must_use]
    fn tools<T>(self, tools: T) -> impl CandleAgentRoleBuilder
    where
        T: Into<ZeroOneOrMany<ToolInfo>>;

    /// Set MCP server - EXACT syntax: .mcp_server::<Stdio>().bin("/path").init("command")
    #[must_use]
    fn mcp_server<T>(self) -> impl CandleMcpServerBuilder
    where
        T: 'static;

    /// Add MCP server config - internal method for MCP server builder
    #[must_use]
    fn add_mcp_server_config(self, config: McpServerConfig) -> impl CandleAgentRoleBuilder;

    /// Set chunk handler - EXACT syntax: .on_chunk(|chunk| async move { chunk })
    #[must_use]
    fn on_chunk<F, Fut>(self, handler: F) -> impl CandleAgentRoleBuilder
    where
        F: Fn(CandleMessageChunk) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CandleMessageChunk> + Send + 'static;

    /// Set tool result handler - EXACT syntax: .on_tool_result(|results| async move { ... })
    #[must_use]
    fn on_tool_result<F, Fut>(self, handler: F) -> impl CandleAgentRoleBuilder
    where
        F: Fn(&[String]) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static;

    /// Set conversation turn handler - EXACT syntax: .on_conversation_turn(|conversation, agent| async move { ... })
    #[must_use]
    fn on_conversation_turn<F, Fut>(self, handler: F) -> impl CandleAgentRoleBuilder
    where
        F: Fn(&CandleAgentConversation, &CandleAgentRoleAgent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>>
            + Send
            + 'static;

    /// Convert to agent - EXACT syntax: .into_agent()
    /// 
    /// # Errors
    /// Returns AgentError::Config if default model is not registered in the global registry
    fn into_agent(self) -> Result<impl CandleAgentBuilder, AgentError>;

    /// Set conversation history - EXACT syntax: .conversation_history(...)
    #[must_use]
    fn conversation_history(
        self,
        history: impl ConversationHistoryArgs,
    ) -> impl CandleAgentRoleBuilder;

    /// Chat with closure - EXACT syntax: .chat(|conversation| ChatLoop)
    fn chat<F, Fut>(
        self,
        handler: F,
    ) -> Result<Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>, AgentError>
    where
        F: Fn(&CandleAgentConversation) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CandleChatLoop> + Send + 'static;

    /// Chat with message - EXACT syntax: .chat_with_message("message")
    fn chat_with_message(
        self,
        message: impl Into<String>,
    ) -> Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>;
}

/// MCP server builder for fluent chaining
pub trait CandleMcpServerBuilder: Sized + Send {
    /// Set binary path - EXACT syntax: .bin("/path/to/binary")
    #[must_use]
    fn bin(self, path: impl Into<String>) -> impl CandleMcpServerBuilder;

    /// Initialize with command - EXACT syntax: .init("command args")
    #[must_use]
    fn init(self, command: impl Into<String>) -> impl CandleAgentRoleBuilder;
}

/// Agent builder trait (PUBLIC API)
pub trait CandleAgentBuilder: Sized + Send + Sync {
    /// Set text-to-text model - EXACT syntax: .model(TextToTextModel)
    #[must_use]
    fn model(self, model: TextToTextModel) -> impl CandleAgentBuilder;

    /// Set text embedding model - EXACT syntax: .embedding_model(TextEmbeddingModel)
    #[must_use]
    fn embedding_model(self, model: TextEmbeddingModel) -> impl CandleAgentBuilder;

    /// Set temperature - EXACT syntax: .temperature(1.0)
    #[must_use]
    fn temperature(self, temp: f64) -> impl CandleAgentBuilder;

    /// Set max tokens - EXACT syntax: .max_tokens(8000)
    #[must_use]
    fn max_tokens(self, max: u64) -> impl CandleAgentBuilder;

    /// Set stop sequences - EXACT syntax: .stop_sequences(vec!["\n\n".to_string(), "###".to_string()])
    #[must_use]
    fn stop_sequences(self, sequences: Vec<String>) -> impl CandleAgentBuilder;

    /// Add single stop sequence - EXACT syntax: .add_stop_sequence("\n\n")
    #[must_use]
    fn add_stop_sequence(self, sequence: impl Into<String>) -> impl CandleAgentBuilder;

    /// Set memory read timeout in milliseconds - EXACT syntax: .memory_read_timeout(5000)
    #[must_use]
    fn memory_read_timeout(self, timeout_ms: u64) -> impl CandleAgentBuilder;

    /// Set system prompt - EXACT syntax: .system_prompt("...")
    #[must_use]
    fn system_prompt(self, prompt: impl Into<String>) -> impl CandleAgentBuilder;

    /// Set additional params - EXACT syntax: .additional_params([("key", "value")])
    #[must_use]
    fn additional_params<P2>(self, params: P2) -> impl CandleAgentBuilder
    where
        P2: IntoIterator<Item = (&'static str, &'static str)>;

    /// Set metadata - EXACT syntax: .metadata(meta)
    #[must_use]
    fn metadata<Meta>(self, metadata: Meta) -> impl CandleAgentBuilder
    where
        Meta: IntoIterator<Item = (&'static str, &'static str)>;

    /// Set contexts - EXACT syntax: .context(...)
    #[must_use]
    fn context(
        self,
        context1: CandleContext<CandleFile>,
        context2: CandleContext<CandleFiles>,
        context3: CandleContext<CandleDirectory>,
        context4: CandleContext<CandleGithub>,
    ) -> impl CandleAgentBuilder;

    /// Set tools - EXACT syntax: .tools(tool1, tool2, tool3)
    #[must_use]
    fn tools<T>(self, tools: T) -> impl CandleAgentBuilder
    where
        T: Into<ZeroOneOrMany<ToolInfo>>;

    /// Set MCP server - EXACT syntax: .mcp_server::<Stdio>().bin("/path").init("command")
    #[must_use]
    fn mcp_server<T>(self) -> impl CandleMcpServerBuilder
    where
        T: 'static;

    /// Add MCP server config - internal method for MCP server builder
    #[must_use]
    fn add_mcp_server_config(self, config: McpServerConfig) -> impl CandleAgentBuilder;

    /// Set chunk handler - EXACT syntax: .on_chunk(|chunk| async move { chunk })
    #[must_use]
    fn on_chunk<F, Fut>(self, handler: F) -> impl CandleAgentBuilder
    where
        F: Fn(CandleMessageChunk) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CandleMessageChunk> + Send + 'static;

    /// Set tool result handler - EXACT syntax: .on_tool_result(|results| async move { ... })
    #[must_use]
    fn on_tool_result<F, Fut>(self, handler: F) -> impl CandleAgentBuilder
    where
        F: Fn(&[String]) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static;

    /// Set conversation turn handler - EXACT syntax: .on_conversation_turn(|conversation, agent| async move { ... })
    #[must_use]
    fn on_conversation_turn<F, Fut>(self, handler: F) -> impl CandleAgentBuilder
    where
        F: Fn(&CandleAgentConversation, &CandleAgentRoleAgent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>>
            + Send
            + 'static;

    /// Set conversation history - EXACT syntax from ARCHITECTURE.md
    /// Supports: .conversation_history(CandleMessageRole::User => "content", CandleMessageRole::System => "content", ...)
    #[must_use]
    fn conversation_history(self, history: impl ConversationHistoryArgs)
    -> impl CandleAgentBuilder;

    /// Chat with closure - EXACT syntax: .chat(|conversation| ChatLoop)
    fn chat<F, Fut>(
        self,
        handler: F,
    ) -> Result<Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>, AgentError>
    where
        F: Fn(&CandleAgentConversation) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CandleChatLoop> + Send + 'static;

    /// Chat with message - EXACT syntax: .chat_with_message("message")
    fn chat_with_message(
        self,
        message: impl Into<String>,
    ) -> Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>;
}
