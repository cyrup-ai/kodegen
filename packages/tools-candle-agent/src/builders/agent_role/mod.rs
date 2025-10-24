//! Agent role builder - Fluent API for AI agent configuration

mod agent_builder;
mod chat;
mod helpers;
mod role_builder;
mod role_builder_impl;
mod traits;

pub(crate) use crate::capability::registry::{TextEmbeddingModel, TextToTextModel};
pub(crate) use crate::capability::traits::TextToTextCapable;
pub(crate) use crate::domain::agent::core::AgentError;
pub(crate) use crate::domain::agent::role::CandleAgentConversation;
pub(crate) use crate::domain::chat::CandleChatLoop;
pub(crate) use crate::domain::chat::message::{CandleMessageChunk, CandleMessageRole};
pub(crate) use crate::domain::completion::CandleCompletionChunk;
pub(crate) use crate::domain::completion::types::ToolInfo;
pub(crate) use crate::domain::context::provider::{
    CandleContext, CandleDirectory, CandleFile, CandleFiles, CandleGithub,
};
pub(crate) use crate::domain::prompt::CandlePrompt;
pub use agent_builder::{AgentDebugInfo, CandleAgentBuilderImpl};
pub(crate) use cyrup_sugars::ZeroOneOrMany;
pub use helpers::{CandleAgentRoleAgent, CandleFluentAi, ConversationHistoryArgs};
pub use role_builder::CandleAgentRoleBuilderImpl;
pub use role_builder_impl::{CandleMcpServerBuilderImpl, McpServerConfig};
pub(crate) use serde_json;
pub(crate) use std::pin::Pin;
pub(crate) use std::sync::Arc;
pub(crate) use tokio_stream::{Stream, StreamExt};
pub use traits::{CandleAgentBuilder, CandleAgentRoleBuilder, CandleMcpServerBuilder};

pub(crate) type OnChunkHandler = Arc<
    dyn Fn(
            CandleMessageChunk,
        ) -> Pin<Box<dyn std::future::Future<Output = CandleMessageChunk> + Send>>
        + Send
        + Sync,
>;
pub(crate) type OnToolResultHandler =
    Arc<dyn Fn(&[String]) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;
pub(crate) type OnConversationTurnHandler = Arc<
    dyn Fn(
            &CandleAgentConversation,
            &CandleAgentRoleAgent,
        ) -> Pin<
            Box<
                dyn std::future::Future<
                        Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>,
                    > + Send,
            >,
        > + Send
        + Sync,
>;

pub struct AgentBuilderState {
    pub name: String,
    pub text_to_text_model: TextToTextModel,
    pub text_embedding_model: Option<TextEmbeddingModel>,
    pub temperature: f64,
    pub max_tokens: u64,
    pub memory_read_timeout: u64,
    pub system_prompt: String,
    pub tools: ZeroOneOrMany<ToolInfo>,
    pub context_file: Option<CandleContext<CandleFile>>,
    pub context_files: Option<CandleContext<CandleFiles>>,
    pub context_directory: Option<CandleContext<CandleDirectory>>,
    pub context_github: Option<CandleContext<CandleGithub>>,
    pub additional_params: std::collections::HashMap<String, String>,
    pub metadata: std::collections::HashMap<String, String>,
    pub on_chunk_handler: Option<OnChunkHandler>,
    pub on_tool_result_handler: Option<OnToolResultHandler>,
    pub on_conversation_turn_handler: Option<OnConversationTurnHandler>,
}
