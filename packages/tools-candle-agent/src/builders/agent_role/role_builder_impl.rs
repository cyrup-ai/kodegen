//! MCP stubs and pre-model builder implementations

use super::*;

pub struct CandleMcpServerBuilderImpl<T> {
    pub(super) parent_builder: T,
    pub(super) binary_path: Option<String>,
}

impl<T> CandleMcpServerBuilder for CandleMcpServerBuilderImpl<T>
where
    T: CandleAgentRoleBuilder,
{
    fn bin(mut self, path: impl Into<String>) -> impl CandleMcpServerBuilder {
        self.binary_path = Some(path.into());
        self
    }

    fn init(self, command: impl Into<String>) -> impl CandleAgentRoleBuilder {
        let config = McpServerConfig {
            binary_path: self.binary_path.clone(),
            init_command: command.into(),
        };
        self.parent_builder.add_mcp_server_config(config)
    }
}

/// MCP server configuration
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub binary_path: Option<String>,
    pub init_command: String,
}

impl CandleAgentRoleBuilder for CandleAgentRoleBuilderImpl {
    fn new(name: impl Into<String>) -> impl CandleAgentRoleBuilder {
        CandleAgentRoleBuilderImpl::new(name)
    }

    fn model(self, model: TextToTextModel) -> impl CandleAgentRoleBuilder {
        use crate::capability::registry;
        use crate::domain::model::traits::CandleModel;

        // Get max_tokens from model's ModelInfo
        let model_max_tokens = model
            .info()
            .max_output_tokens
            .map(|t| t.get().into())
            .unwrap_or(2000);

        // Get default embedding model from registry (if available)
        let default_embedding_model =
            registry::get::<TextEmbeddingModel>("dunzhang/stella_en_400M_v5");

        CandleAgentBuilderImpl {
            name: self.name,
            text_to_text_model: model,
            text_embedding_model: self.text_embedding_model.or(default_embedding_model),
            temperature: self.temperature,
            max_tokens: self.max_tokens.unwrap_or(model_max_tokens),
            memory_read_timeout: self.memory_read_timeout,
            system_prompt: self.system_prompt,
            tools: self.tools,
            context_file: self.context_file,
            context_files: self.context_files,
            context_directory: self.context_directory,
            context_github: self.context_github,
            additional_params: self.additional_params,
            metadata: self.metadata,
            on_chunk_handler: self.on_chunk_handler,
            on_tool_result_handler: self.on_tool_result_handler,
            on_conversation_turn_handler: self.on_conversation_turn_handler,
            conversation_history: self.conversation_history,
            stop_sequences: self.stop_sequences,
        }
    }

    fn embedding_model(self, _model: TextEmbeddingModel) -> impl CandleAgentRoleBuilder {
        // For CandleAgentRoleBuilderImpl (no model yet), we can't set embedding model without text model
        // Return self unchanged - user should call .model() first
        self
    }

    /// Set temperature - EXACT syntax: .temperature(1.0)
    fn temperature(mut self, temp: f64) -> impl CandleAgentRoleBuilder {
        self.temperature = temp;
        self
    }

    /// Set max tokens - EXACT syntax: .max_tokens(8000)
    fn max_tokens(mut self, max: u64) -> impl CandleAgentRoleBuilder {
        self.max_tokens = Some(max);
        self
    }

    /// Set memory read timeout in milliseconds - EXACT syntax: .memory_read_timeout(5000)
    fn memory_read_timeout(mut self, timeout_ms: u64) -> impl CandleAgentRoleBuilder {
        self.memory_read_timeout = timeout_ms;
        self
    }

    /// Set system prompt - EXACT syntax: .system_prompt("...")
    fn system_prompt(mut self, prompt: impl Into<String>) -> impl CandleAgentRoleBuilder {
        self.system_prompt = prompt.into();
        self
    }

    /// Set additional params - EXACT syntax: .additional_params([("key", "value")])
    fn additional_params<P>(mut self, params: P) -> impl CandleAgentRoleBuilder
    where
        P: IntoIterator<Item = (&'static str, &'static str)>,
    {
        self.additional_params.extend(
            params
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    /// Set metadata - EXACT syntax: .metadata([("key", "value")])
    fn metadata<Meta>(mut self, metadata: Meta) -> impl CandleAgentRoleBuilder
    where
        Meta: IntoIterator<Item = (&'static str, &'static str)>,
    {
        self.metadata.extend(
            metadata
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    /// Set contexts - EXACT syntax: .context(CandleContext::<CandleFile>::of("/path"), ...)
    fn context(
        mut self,
        context1: CandleContext<CandleFile>,
        context2: CandleContext<CandleFiles>,
        context3: CandleContext<CandleDirectory>,
        context4: CandleContext<CandleGithub>,
    ) -> impl CandleAgentRoleBuilder {
        self.context_file = Some(context1);
        self.context_files = Some(context2);
        self.context_directory = Some(context3);
        self.context_github = Some(context4);
        self
    }

    /// Set tools - EXACT syntax: .tools(tool1, tool2, tool3)
    fn tools<T>(mut self, tools: T) -> impl CandleAgentRoleBuilder
    where
        T: Into<ZeroOneOrMany<ToolInfo>>,
    {
        self.tools = tools.into();
        self
    }

    /// Set MCP server - EXACT syntax: .mcp_server::<Stdio>().bin("/path").init("command")
    fn mcp_server<T>(self) -> impl CandleMcpServerBuilder
    where
        T: 'static,
    {
        CandleMcpServerBuilderImpl {
            parent_builder: self,
            binary_path: None,
        }
    }

    /// Add MCP server config - internal method for MCP server builder
    fn add_mcp_server_config(self, _config: McpServerConfig) -> impl CandleAgentRoleBuilder {
        // MCP servers are handled through tools
        self
    }

    fn on_chunk<F, Fut>(mut self, handler: F) -> impl CandleAgentRoleBuilder
    where
        F: Fn(CandleMessageChunk) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CandleMessageChunk> + Send + 'static,
    {
        let wrapped = move |chunk: CandleMessageChunk| {
            Box::pin(handler(chunk))
                as Pin<Box<dyn std::future::Future<Output = CandleMessageChunk> + Send>>
        };
        self.on_chunk_handler = Some(Arc::new(wrapped));
        self
    }

    fn on_tool_result<F, Fut>(self, _handler: F) -> impl CandleAgentRoleBuilder
    where
        F: Fn(&[String]) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // Cannot set handler without a model - return self unchanged
        self
    }

    /// Set conversation turn handler - EXACT syntax: .on_conversation_turn(|conversation, agent| async move { ... })
    fn on_conversation_turn<F, Fut>(self, _handler: F) -> impl CandleAgentRoleBuilder
    where
        F: Fn(&CandleAgentConversation, &CandleAgentRoleAgent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>>
            + Send
            + 'static,
    {
        // Cannot set handler without a model - return self unchanged
        self
    }

    fn conversation_history(
        mut self,
        history: impl ConversationHistoryArgs,
    ) -> impl CandleAgentRoleBuilder {
        self.conversation_history = history.into_history();
        self
    }

    /// Chat with async closure - EXACT syntax: .chat(|conversation| async { ChatLoop })
    fn chat<F, Fut>(
        self,
        _handler: F,
    ) -> Result<Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>, AgentError>
    where
        F: Fn(&CandleAgentConversation) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CandleChatLoop> + Send + 'static,
    {
        Ok(Box::pin(crate::async_stream::spawn_stream(
            |sender| async move {
                let _ = sender.send(CandleMessageChunk::Error(
                    "No provider configured".to_string(),
                ));
            },
        )))
    }

    fn chat_with_message(
        self,
        message: impl Into<String>,
    ) -> Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>> {
        let msg = message.into();
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let _ = sender.send(CandleMessageChunk::Error(format!(
                    "No provider configured for message: {}",
                    msg
                )));
            },
        ))
    }

    /// Convert to agent - EXACT syntax: .into_agent()
    fn into_agent(self) -> Result<impl CandleAgentBuilder, AgentError> {
        use crate::capability::registry;
        use crate::domain::model::traits::CandleModel;

        // Get default text-to-text model if not set
        let text_model = match self.text_to_text_model {
            Some(model) => model,
            None => {
                registry::get::<TextToTextModel>("qwen-3")
                    .ok_or_else(|| AgentError::Config(
                        "Default 'qwen-3' model not found in registry. \
                         Ensure registry is properly initialized at startup or specify a model explicitly using .model()".to_string()
                    ))?
            }
        };

        // Get max_tokens from model's ModelInfo
        let model_max_tokens = text_model
            .info()
            .max_output_tokens
            .map(|t| t.get().into())
            .unwrap_or(2000);

        // Get default embedding model from registry (if available)
        let embedding_model = self
            .text_embedding_model
            .or_else(|| registry::get::<TextEmbeddingModel>("dunzhang/stella_en_400M_v5"));

        Ok(CandleAgentBuilderImpl {
            name: self.name,
            text_to_text_model: text_model,
            text_embedding_model: embedding_model,
            temperature: self.temperature,
            max_tokens: self.max_tokens.unwrap_or(model_max_tokens),
            memory_read_timeout: self.memory_read_timeout,
            system_prompt: self.system_prompt,
            tools: self.tools,
            context_file: self.context_file,
            context_files: self.context_files,
            context_directory: self.context_directory,
            context_github: self.context_github,
            additional_params: self.additional_params,
            metadata: self.metadata,
            on_chunk_handler: self.on_chunk_handler,
            on_tool_result_handler: self.on_tool_result_handler,
            on_conversation_turn_handler: self.on_conversation_turn_handler,
            conversation_history: self.conversation_history,
            stop_sequences: self.stop_sequences,
        })
    }
}
