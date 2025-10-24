//! Chat implementation for CandleAgentBuilder
//!
//! This module coordinates the chat functionality across focused submodules:
//! - builder_methods: Simple builder pattern setters
//! - handler_registration: Handler wrapping and Arc management
//! - memory_ops: Memory initialization, context loading, and storage
//!
//! Chat orchestration is delegated to domain::chat::session

mod builder_methods;
mod handler_registration;
mod memory_ops;

use super::*;
use std::sync::Arc;
use tokio_stream::StreamExt;

impl CandleAgentBuilder for CandleAgentBuilderImpl {
    fn model(self, model: TextToTextModel) -> impl CandleAgentBuilder {
        builder_methods::set_model(self, model)
    }

    fn embedding_model(self, model: TextEmbeddingModel) -> impl CandleAgentBuilder {
        builder_methods::set_embedding_model(self, model)
    }

    fn temperature(self, temp: f64) -> impl CandleAgentBuilder {
        builder_methods::set_temperature(self, temp)
    }

    fn max_tokens(self, max: u64) -> impl CandleAgentBuilder {
        builder_methods::set_max_tokens(self, max)
    }

    fn stop_sequences(self, sequences: Vec<String>) -> impl CandleAgentBuilder {
        builder_methods::set_stop_sequences(self, sequences)
    }

    fn add_stop_sequence(self, sequence: impl Into<String>) -> impl CandleAgentBuilder {
        builder_methods::add_stop_sequence_impl(self, sequence.into())
    }

    fn memory_read_timeout(self, timeout_ms: u64) -> impl CandleAgentBuilder {
        builder_methods::set_memory_read_timeout(self, timeout_ms)
    }

    fn system_prompt(self, prompt: impl Into<String>) -> impl CandleAgentBuilder {
        builder_methods::set_system_prompt(self, prompt.into())
    }

    fn additional_params<P2>(self, params: P2) -> impl CandleAgentBuilder
    where
        P2: IntoIterator<Item = (&'static str, &'static str)>,
    {
        builder_methods::set_additional_params(self, params)
    }

    fn metadata<Meta>(self, metadata: Meta) -> impl CandleAgentBuilder
    where
        Meta: IntoIterator<Item = (&'static str, &'static str)>,
    {
        builder_methods::set_metadata(self, metadata)
    }

    fn context(
        self,
        context1: CandleContext<CandleFile>,
        context2: CandleContext<CandleFiles>,
        context3: CandleContext<CandleDirectory>,
        context4: CandleContext<CandleGithub>,
    ) -> impl CandleAgentBuilder {
        builder_methods::set_context(self, context1, context2, context3, context4)
    }

    fn tools<T>(self, tools: T) -> impl CandleAgentBuilder
    where
        T: Into<ZeroOneOrMany<ToolInfo>>,
    {
        builder_methods::set_tools(self, tools)
    }

    fn mcp_server<T>(self) -> impl CandleMcpServerBuilder
    where
        T: 'static,
    {
        CandleMcpServerBuilderImpl {
            parent_builder: self,
            binary_path: None,
        }
    }

    fn add_mcp_server_config(self, config: McpServerConfig) -> impl CandleAgentBuilder {
        builder_methods::add_mcp_server_config_impl(self, config)
    }

    fn on_chunk<F, Fut>(mut self, handler: F) -> impl CandleAgentBuilder
    where
        F: Fn(CandleMessageChunk) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CandleMessageChunk> + Send + 'static,
    {
        self.on_chunk_handler = Some(handler_registration::wrap_chunk_handler(handler));
        self
    }

    fn on_tool_result<F, Fut>(mut self, handler: F) -> impl CandleAgentBuilder
    where
        F: Fn(&[String]) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.on_tool_result_handler = Some(handler_registration::wrap_tool_result_handler(handler));
        self
    }

    fn on_conversation_turn<F, Fut>(mut self, handler: F) -> impl CandleAgentBuilder
    where
        F: Fn(&CandleAgentConversation, &CandleAgentRoleAgent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>>
            + Send
            + 'static,
    {
        self.on_conversation_turn_handler = Some(
            handler_registration::wrap_conversation_turn_handler(handler),
        );
        self
    }

    fn conversation_history(
        self,
        _history: impl ConversationHistoryArgs,
    ) -> impl CandleAgentBuilder {
        self
    }

    fn chat<F, Fut>(
        self,
        handler: F,
    ) -> Result<Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>, AgentError>
    where
        F: FnOnce(&CandleAgentConversation) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = CandleChatLoop> + Send + 'static,
    {
        // Build configurations
        let model_config = self.build_model_config();
        let chat_config = self.build_chat_config();

        // Extract all state from builder
        let provider = self.text_to_text_model;
        let embedding_model = self.text_embedding_model;
        let tools: Arc<[ToolInfo]> = Vec::from(self.tools).into();
        let metadata = self.metadata;
        let conversation_history = self.conversation_history;

        // Extract handlers
        let on_chunk_handler = self.on_chunk_handler;
        let on_tool_result_handler = self.on_tool_result_handler;
        let on_conversation_turn_handler = self.on_conversation_turn_handler;

        // Extract context sources
        let context_file = self.context_file;
        let context_files = self.context_files;
        let context_directory = self.context_directory;
        let context_github = self.context_github;

        Ok(Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                // Initialize memory manager if embedding model available
                let memory = if let Some(ref emb_model) = embedding_model {
                    match memory_ops::initialize_memory_coordinator(emb_model).await {
                        Ok(mgr) => mgr,
                        Err(e) => {
                            let _ = sender.send(CandleMessageChunk::Error(e));
                            return;
                        }
                    }
                } else {
                    let _ = sender.send(CandleMessageChunk::Error(
                        "Embedding model required for memory system".to_string(),
                    ));
                    return;
                };

                // DELEGATE to domain::chat::session with raw context sources
                let config = crate::domain::chat::session::ChatSessionConfig {
                    model_config,
                    chat_config,
                    provider,
                    memory,
                    tools,
                    metadata,
                };
                let contexts = crate::domain::chat::session::ChatSessionContexts {
                    context_file,
                    context_files,
                    context_directory,
                    context_github,
                };
                let handlers = crate::domain::chat::session::ChatSessionHandlers {
                    on_chunk_handler,
                    on_tool_result_handler,
                    on_conversation_turn_handler,
                };

                let session_stream = crate::domain::chat::session::execute_chat_session(
                    config,
                    contexts,
                    conversation_history,
                    handler,
                    handlers,
                )
                .await;

                // Forward all chunks from session to sender
                tokio::pin!(session_stream);
                while let Some(chunk) = session_stream.next().await {
                    let _ = sender.send(chunk);
                }
            },
        )))
    }

    fn chat_with_message(
        self,
        message: impl Into<String>,
    ) -> Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>> {
        let msg = message.into();

        // Use the main chat method with a simple handler
        match CandleAgentBuilder::chat(self, move |_| {
            let msg = msg.clone();
            async move { CandleChatLoop::UserPrompt(msg) }
        }) {
            Ok(stream) => stream,
            Err(e) => Box::pin(crate::async_stream::spawn_stream(
                move |sender| async move {
                    let _ = sender.send(CandleMessageChunk::Error(format!("Chat failed: {}", e)));
                },
            )),
        }
    }
}
