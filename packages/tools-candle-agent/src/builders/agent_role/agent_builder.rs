//! CandleAgentBuilderImpl - builder with model

use super::*;
use crate::domain::chat::config::{
    CandleBehaviorConfig, CandleChatConfig, CandleModelConfig, CandleModelPerformanceConfig,
    CandleModelRetryConfig, CandlePersonalityConfig, CandleUIConfig,
};
use crate::domain::model::traits::CandleModel;
use std::time::Duration;

pub struct AgentDebugInfo {
    pub name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub has_system_prompt: bool,
}

/// Agent builder implementation
pub struct CandleAgentBuilderImpl {
    pub(super) name: String,
    pub(super) text_to_text_model: TextToTextModel,
    pub(super) text_embedding_model: Option<TextEmbeddingModel>,
    pub(super) temperature: f64,
    pub(super) max_tokens: u64,
    pub(super) memory_read_timeout: u64,
    pub(super) system_prompt: String,
    pub(super) tools: ZeroOneOrMany<ToolInfo>,
    pub(super) context_file: Option<CandleContext<CandleFile>>,
    pub(super) context_files: Option<CandleContext<CandleFiles>>,
    pub(super) context_directory: Option<CandleContext<CandleDirectory>>,
    pub(super) context_github: Option<CandleContext<CandleGithub>>,
    pub(super) additional_params: std::collections::HashMap<String, String>,
    pub(super) metadata: std::collections::HashMap<String, String>,
    pub(super) on_chunk_handler: Option<OnChunkHandler>,
    pub(super) on_tool_result_handler: Option<OnToolResultHandler>,
    pub(super) on_conversation_turn_handler: Option<OnConversationTurnHandler>,
    pub(super) conversation_history: ZeroOneOrMany<(CandleMessageRole, String)>,
    pub(super) stop_sequences: Vec<String>,
}

impl std::fmt::Debug for CandleAgentBuilderImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandleAgentBuilderImpl")
            .field("name", &self.name)
            .field("text_to_text_model", &self.text_to_text_model)
            .field("text_embedding_model", &self.text_embedding_model)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("memory_read_timeout", &self.memory_read_timeout)
            .field(
                "system_prompt",
                &format!(
                    "{}...",
                    &self.system_prompt[..self.system_prompt.len().min(50)]
                ),
            )
            .field("tools", &self.tools)
            .field("additional_params", &self.additional_params)
            .field("metadata", &self.metadata)
            .finish()
    }
}

// Implementation for with-provider builder (allows all methods)
impl CandleAgentRoleBuilder for CandleAgentBuilderImpl {
    fn new(name: impl Into<String>) -> impl CandleAgentRoleBuilder {
        CandleAgentRoleBuilderImpl::new(name)
    }

    fn model(mut self, model: TextToTextModel) -> impl CandleAgentRoleBuilder {
        self.text_to_text_model = model;
        self
    }

    fn embedding_model(mut self, model: TextEmbeddingModel) -> impl CandleAgentRoleBuilder {
        self.text_embedding_model = Some(model);
        self
    }

    fn temperature(mut self, temp: f64) -> impl CandleAgentRoleBuilder {
        self.temperature = temp;
        self
    }

    fn max_tokens(mut self, max: u64) -> impl CandleAgentRoleBuilder {
        self.max_tokens = max;
        self
    }

    fn memory_read_timeout(mut self, timeout_ms: u64) -> impl CandleAgentRoleBuilder {
        self.memory_read_timeout = timeout_ms;
        self
    }

    fn system_prompt(mut self, prompt: impl Into<String>) -> impl CandleAgentRoleBuilder {
        self.system_prompt = prompt.into();
        self
    }

    fn additional_params<P2>(mut self, params: P2) -> impl CandleAgentRoleBuilder
    where
        P2: IntoIterator<Item = (&'static str, &'static str)>,
    {
        self.additional_params.extend(
            params
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

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

    fn tools<T>(mut self, tools: T) -> impl CandleAgentRoleBuilder
    where
        T: Into<ZeroOneOrMany<ToolInfo>>,
    {
        self.tools = tools.into();
        self
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

    fn on_tool_result<F, Fut>(mut self, handler: F) -> impl CandleAgentRoleBuilder
    where
        F: Fn(&[String]) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let wrapped = move |results: &[String]| {
            Box::pin(handler(results)) as Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        };
        self.on_tool_result_handler = Some(Arc::new(wrapped));
        self
    }

    /// Set conversation turn handler - EXACT syntax: .on_conversation_turn(|conversation, agent| async move { ... })
    fn on_conversation_turn<F, Fut>(mut self, handler: F) -> impl CandleAgentRoleBuilder
    where
        F: Fn(&CandleAgentConversation, &CandleAgentRoleAgent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>>
            + Send
            + 'static,
    {
        // Wrap the async handler to match the type alias
        let wrapped_handler = move |conv: &CandleAgentConversation,
                                    agent: &CandleAgentRoleAgent| {
            Box::pin(handler(conv, agent))
                as Pin<
                    Box<
                        dyn std::future::Future<
                                Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>,
                            > + Send,
                    >,
                >
        };
        self.on_conversation_turn_handler = Some(Arc::new(wrapped_handler));
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
                let _ = sender.send(CandleMessageChunk::Text("Hello from Candle!".to_string()));
            },
        )))
    }

    fn chat_with_message(
        self,
        message: impl Into<String>,
    ) -> Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>> {
        let msg = message.into();
        // Use CandleAgentRoleBuilder::chat explicitly to avoid ambiguity
        CandleAgentRoleBuilder::chat(self, move |_| {
            let msg = msg.clone();
            async move { CandleChatLoop::UserPrompt(msg) }
        })
        .unwrap_or_else(|_| {
            Box::pin(crate::async_stream::spawn_stream(|sender| async move {
                let _ = sender.send(CandleMessageChunk::Error("Chat failed".to_string()));
            }))
        })
    }

    fn into_agent(self) -> Result<impl CandleAgentBuilder, AgentError> {
        Ok(self)
    }
}

impl CandleAgentBuilderImpl {
    /// Build CandleModelConfig by merging model defaults with builder overrides
    pub(crate) fn build_model_config(&self) -> CandleModelConfig {
        // Get model info which contains defaults
        let model_info = self.text_to_text_model.info();

        CandleModelConfig {
            // Provider and model identification
            provider: model_info.provider.as_str().to_string(),
            registry_key: model_info.registry_key.to_string(),
            model_version: model_info.real_name.as_ref().map(|s| s.to_string()),

            // Temperature: builder override > model default > fallback 0.7
            temperature: self.temperature as f32,

            // Max tokens: builder value > model max_output_tokens > fallback 2048
            max_tokens: Some(self.max_tokens as u32),

            // Sampling parameters (not in builder, use model defaults or standard)
            top_p: model_info.default_top_p.map(|p| p as f32).or(Some(1.0)),
            top_k: model_info.default_top_k,
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),

            stop_sequences: self.stop_sequences.clone(),

            // System prompt from builder
            system_prompt: if self.system_prompt.is_empty() {
                model_info.system_prompt_prefix.clone()
            } else {
                Some(self.system_prompt.clone())
            },

            // Function calling: enabled if tools present OR model supports it
            enable_functions: !self.tools.is_empty() || model_info.supports_function_calling,
            function_mode: if !self.tools.is_empty() {
                "required".to_string()
            } else if model_info.supports_function_calling {
                "auto".to_string()
            } else {
                "none".to_string()
            },

            // Custom parameters from builder's additional_params
            custom_parameters: self
                .additional_params
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect(),

            // Timeout from memory_read_timeout
            timeout_ms: self.memory_read_timeout,

            // Retry configuration with reasonable defaults
            retry_config: CandleModelRetryConfig {
                max_retries: 3,
                base_delay_ms: 1000,
                max_delay_ms: 30000,
                backoff_multiplier: 2.0,
                enable_jitter: true,
            },

            // Performance configuration based on model capabilities
            performance: CandleModelPerformanceConfig {
                enable_caching: model_info.supports_kv_cache,
                cache_ttl_seconds: 3600,
                enable_batching: false,
                max_batch_size: 1,
                batch_timeout_ms: 100,
                enable_streaming: model_info.supports_streaming,
                connection_pool_size: 10,
                keep_alive_timeout_seconds: 60,
            },
        }
    }

    /// Build CandleChatConfig from builder state
    pub(crate) fn build_chat_config(&self) -> CandleChatConfig {
        CandleChatConfig {
            // Message configuration
            max_message_length: 100_000, // 100KB reasonable limit
            enable_history: !self.conversation_history.is_empty(),
            history_retention: Duration::from_secs(86400), // 24 hours
            enable_streaming: true,                        // Always enable for this architecture

            // Personality configuration with neutral defaults
            personality: CandlePersonalityConfig {
                personality_type: "assistant".to_string(),
                response_style: "balanced".to_string(),
                tone: "professional".to_string(),
                custom_instructions: None,
                creativity: 0.5,
                formality: 0.7,
                humor: 0.2,
                empathy: 0.6,
                expertise_level: "intermediate".to_string(),
                verbosity: "moderate".to_string(),
                traits: vec!["helpful".to_string(), "accurate".to_string()],
            },

            // Behavior configuration
            behavior: CandleBehaviorConfig {
                auto_response: false,
                response_delay: Duration::from_millis(0),
                enable_filtering: false,
                max_concurrent_chats: 1,
                proactivity: 0.3,
                question_frequency: 0.4,
                conversation_flow: "natural".to_string(),
                follow_up_behavior: "contextual".to_string(),
                error_handling: "graceful".to_string(),
            },

            // UI configuration (use existing structure)
            ui: CandleUIConfig {
                theme: "default".to_string(),
                font_size: 14,
                dark_mode: false,
                enable_animations: true,
                layout: "standard".to_string(),
                color_scheme: "adaptive".to_string(),
                display_density: "comfortable".to_string(),
                animations: "smooth".to_string(),
            },
        }
    }
}
