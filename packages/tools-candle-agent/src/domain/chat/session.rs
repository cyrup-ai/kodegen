//! Chat session orchestration executor

use futures::future::BoxFuture;
use std::collections::HashMap;
use std::fmt::Write;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};

// Context types (use provider:: to get the concrete struct, not the trait)
use crate::domain::context::provider::{
    CandleContext, CandleDirectory, CandleFile, CandleFiles, CandleGithub,
};

// Memory helper functions (copied from builders since they're not publicly exported)

// Import domain types
use crate::builders::agent_role::CandleAgentRoleAgent;
use crate::domain::agent::core::AGENT_STATS;
use crate::domain::agent::role::CandleAgentConversation;
use crate::domain::chat::{
    config::{CandleChatConfig, CandleModelConfig},
    r#loop::CandleChatLoop,
    message::{CandleMessageChunk, CandleMessageRole},
};
use crate::domain::completion::CandleCompletionChunk;
use crate::domain::completion::CandleCompletionParams;
use crate::domain::prompt::CandlePrompt;
use crate::domain::tool::CandleToolRouter;

use crate::builders::agent_role::AgentBuilderState;
use crate::capability::registry::TextToTextModel;
use crate::capability::traits::TextToTextCapable;
use crate::domain::memory::primitives::node::MemoryNode as DomainMemoryNode;
use crate::domain::memory::primitives::types::MemoryTypeEnum as DomainMemoryTypeEnum;
use crate::memory::MemoryMetadata;
use crate::memory::core::manager::coordinator::MemoryCoordinator;
use crate::memory::core::manager::surreal::MemoryManager; // Trait must be in scope
use crate::memory::primitives::node::MemoryNode as CoreMemoryNode;
use crate::memory::primitives::types::{MemoryContent, MemoryTypeEnum as CoreMemoryTypeEnum};

use cyrup_sugars::collections::ZeroOneOrMany;
use crate::domain::completion::types::ToolInfo;

// Type aliases for complex callback types
type OnChunkHandler =
    Arc<dyn Fn(CandleMessageChunk) -> BoxFuture<'static, CandleMessageChunk> + Send + Sync>;
type OnToolResultHandler = Arc<dyn Fn(&[String]) -> BoxFuture<'static, ()> + Send + Sync>;
type OnConversationTurnHandler = Arc<
    dyn Fn(
            &CandleAgentConversation,
            &crate::builders::agent_role::CandleAgentRoleAgent,
        ) -> BoxFuture<'static, Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>>
        + Send
        + Sync,
>;

/// Configuration bundle for chat session execution
pub struct ChatSessionConfig<S> {
    pub model_config: CandleModelConfig,
    pub chat_config: CandleChatConfig,
    pub provider: TextToTextModel,
    pub memory: Arc<MemoryCoordinator>,
    pub tools: Arc<[ToolInfo]>,
    pub metadata: HashMap<String, String, S>,
}

/// Context sources bundle for chat session
pub struct ChatSessionContexts {
    pub context_file: Option<CandleContext<CandleFile>>,
    pub context_files: Option<CandleContext<CandleFiles>>,
    pub context_directory: Option<CandleContext<CandleDirectory>>,
    pub context_github: Option<CandleContext<CandleGithub>>,
}

/// Callback handlers bundle for chat session
pub struct ChatSessionHandlers {
    pub on_chunk_handler: Option<OnChunkHandler>,
    pub on_tool_result_handler: Option<OnToolResultHandler>,
    pub on_conversation_turn_handler: Option<OnConversationTurnHandler>,
}

// Helper functions for memory operations

fn format_memory_context(memories: &[DomainMemoryNode], max_chars: usize) -> String {
    let mut result = String::from("## Relevant Context\n\n");
    let mut current_len = result.len();

    for memory in memories {
        let content = memory.content().to_string();
        let source = memory
            .metadata
            .custom
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let entry = format!("- [{source}]: {content}\n");

        if current_len + entry.len() > max_chars {
            break;
        }

        result.push_str(&entry);
        current_len += entry.len();
    }

    result
}

/// Load documents from a context stream into memory using `MemoryManager` API
async fn load_context_stream(
    stream: Pin<Box<dyn Stream<Item = crate::domain::context::CandleDocument> + Send>>,
    memory: Arc<MemoryCoordinator>,
    metadata: HashMap<String, String>,
    context_tag: &str,
) {
    tokio::pin!(stream);
    while let Some(doc) = stream.next().await {
        // Create CoreMemoryNode following MemoryManager pattern
        let content = MemoryContent::new(&doc.data);
        let mut node = CoreMemoryNode::new(CoreMemoryTypeEnum::Semantic, content);

        // Set metadata fields directly on node.metadata (public fields)
        node.metadata.user_id = metadata.get("user_id").cloned();
        node.metadata.agent_id = metadata.get("agent_id").cloned();
        node.metadata.context = "session_context".to_string();
        node.metadata.category = "context".to_string();
        node.metadata.source = doc
            .additional_props
            .get("path")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string);
        node.metadata.importance = 0.5;
        node.metadata.tags.push(context_tag.to_string());

        // Use LOW-LEVEL MemoryManager trait method (returns PendingMemory Future)
        let pending = memory.create_memory(node);
        if let Err(e) = pending.await {
            log::warn!("Failed to load context document from {context_tag}: {e:?}");
        }
    }
}

/// Handle Break loop case
fn process_break_loop() -> CandleMessageChunk {
    CandleMessageChunk::Complete {
        text: String::new(),
        finish_reason: Some("break".to_string()),
        usage: None,
        token_count: None,
        elapsed_secs: None,
        tokens_per_sec: None,
    }
}

/// Initialize tool router (using workspace MCP infrastructure)
async fn initialize_tool_router(
    _sender: &tokio::sync::mpsc::UnboundedSender<CandleMessageChunk>,
) -> Option<CandleToolRouter> {
    // Create router with no remote MCP client (local tools only)
    let mut router = CandleToolRouter::new(None);
    router.initialize().await.ok()?;
    Some(router)
}

/// Search memory and format context
async fn search_and_format_memory(memory: &Arc<MemoryCoordinator>, user_message: &str) -> String {
    match memory.search_memories(user_message, 10, None).await {
        Ok(memories) => {
            if memories.is_empty() {
                String::new()
            } else {
                format_memory_context(&memories, 2000)
            }
        }
        Err(e) => {
            log::warn!("Memory search failed: {e:?}");
            String::new()
        }
    }
}

/// Build system prompt with personality traits and custom instructions
fn build_system_prompt(model_config: &CandleModelConfig, chat_config: &CandleChatConfig) -> String {
    let mut system_prompt = model_config.system_prompt.clone().unwrap_or_default();

    if let Some(custom) = &chat_config.personality.custom_instructions {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(custom);
    }

    let _ = write!(
        system_prompt,
        "\n\nPersonality: {} (creativity: {:.1}, formality: {:.1}, empathy: {:.1})",
        chat_config.personality.personality_type,
        chat_config.personality.creativity,
        chat_config.personality.formality,
        chat_config.personality.empathy
    );

    system_prompt
}

/// Build prompt with personality and memory context
fn build_prompt_with_context(
    model_config: &CandleModelConfig,
    chat_config: &CandleChatConfig,
    memory_context: &str,
    user_message: &str,
) -> String {
    let system_prompt = build_system_prompt(model_config, chat_config);

    if memory_context.is_empty() {
        format!("{system_prompt}\n\nUser: {user_message}")
    } else {
        format!("{system_prompt}\n\n{memory_context}\n\nUser: {user_message}")
    }
}

/// Load all context sources in parallel
fn load_all_contexts<S>(
    memory: &Arc<MemoryCoordinator>,
    metadata: &HashMap<String, String, S>,
    context_file: Option<CandleContext<CandleFile>>,
    context_files: Option<CandleContext<CandleFiles>>,
    context_directory: Option<CandleContext<CandleDirectory>>,
    context_github: Option<CandleContext<CandleGithub>>,
) -> Vec<tokio::task::JoinHandle<()>>
where
    S: std::hash::BuildHasher,
{
    let mut load_tasks = Vec::new();

    // Extract metadata values once before spawning tasks
    let user_id = metadata.get("user_id").cloned();
    let agent_id = metadata.get("agent_id").cloned();

    if let Some(ctx) = context_file {
        let mem = memory.clone();
        let user_id = user_id.clone();
        let agent_id = agent_id.clone();
        load_tasks.push(tokio::spawn(async move {
            let mut meta = HashMap::new();
            if let Some(uid) = user_id {
                meta.insert("user_id".to_string(), uid);
            }
            if let Some(aid) = agent_id {
                meta.insert("agent_id".to_string(), aid);
            }
            load_context_stream(ctx.load(), mem, meta, "context_file").await;
        }));
    }

    if let Some(ctx) = context_files {
        let mem = memory.clone();
        let user_id = user_id.clone();
        let agent_id = agent_id.clone();
        load_tasks.push(tokio::spawn(async move {
            let mut meta = HashMap::new();
            if let Some(uid) = user_id {
                meta.insert("user_id".to_string(), uid);
            }
            if let Some(aid) = agent_id {
                meta.insert("agent_id".to_string(), aid);
            }
            load_context_stream(ctx.load(), mem, meta, "context_files").await;
        }));
    }

    if let Some(ctx) = context_directory {
        let mem = memory.clone();
        let user_id = user_id.clone();
        let agent_id = agent_id.clone();
        load_tasks.push(tokio::spawn(async move {
            let mut meta = HashMap::new();
            if let Some(uid) = user_id {
                meta.insert("user_id".to_string(), uid);
            }
            if let Some(aid) = agent_id {
                meta.insert("agent_id".to_string(), aid);
            }
            load_context_stream(ctx.load(), mem, meta, "context_directory").await;
        }));
    }

    if let Some(ctx) = context_github {
        let mem = memory.clone();
        let user_id = user_id.clone();
        let agent_id = agent_id.clone();
        load_tasks.push(tokio::spawn(async move {
            let mut meta = HashMap::new();
            if let Some(uid) = user_id {
                meta.insert("user_id".to_string(), uid);
            }
            if let Some(aid) = agent_id {
                meta.insert("agent_id".to_string(), aid);
            }
            load_context_stream(ctx.load(), mem, meta, "context_github").await;
        }));
    }

    load_tasks
}

/// Stream completion chunks and process them with handlers
#[allow(clippy::too_many_arguments)]
async fn stream_and_process_chunks(
    completion_stream: Pin<Box<dyn Stream<Item = CandleCompletionChunk> + Send>>,
    sender: &tokio::sync::mpsc::UnboundedSender<CandleMessageChunk>,
    chat_config: &CandleChatConfig,
    tool_router: Option<&CandleToolRouter>,
    on_chunk_handler: Option<&OnChunkHandler>,
    on_tool_result_handler: Option<&OnToolResultHandler>,
) -> String {
    tokio::pin!(completion_stream);
    let mut assistant_response = String::new();

    while let Some(completion_chunk) = completion_stream.next().await {
        let message_chunk = match completion_chunk {
            CandleCompletionChunk::Text(ref text) => {
                assistant_response.push_str(text);
                CandleMessageChunk::Text(text.clone())
            }
            CandleCompletionChunk::Complete {
                ref text,
                finish_reason,
                usage,
                token_count,
                elapsed_secs,
                tokens_per_sec,
            } => {
                assistant_response.push_str(text);

                // Record completion statistics
                if let Some(token_count) = token_count {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let duration_us = (elapsed_secs.unwrap_or(0.0) * 1_000_000.0) as u64;
                    AGENT_STATS.record_completion(u64::from(token_count), duration_us);
                } else if let Some(usage) = usage {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let duration_us = (elapsed_secs.unwrap_or(0.0) * 1_000_000.0) as u64;
                    AGENT_STATS.record_completion(u64::from(usage.total_tokens), duration_us);
                }

                CandleMessageChunk::Complete {
                    text: text.clone(),
                    finish_reason: finish_reason.map(|f| format!("{f:?}")),
                    usage: usage.map(|u| format!("{u:?}")),
                    token_count,
                    elapsed_secs,
                    tokens_per_sec,
                }
            }
            CandleCompletionChunk::ToolCallStart { id, name } => {
                CandleMessageChunk::ToolCallStart { id, name }
            }
            CandleCompletionChunk::ToolCall {
                id,
                name,
                partial_input,
            } => CandleMessageChunk::ToolCall {
                id,
                name,
                partial_input,
            },
            CandleCompletionChunk::ToolCallComplete { id: _, name, input } => {
                execute_tool_call(&name, &input, tool_router, on_tool_result_handler).await
            }
            CandleCompletionChunk::Error(error) => CandleMessageChunk::Error(error),
        };

        if !chat_config.behavior.response_delay.is_zero() {
            tokio::time::sleep(chat_config.behavior.response_delay).await;
        }

        let final_chunk = if let Some(handler) = on_chunk_handler {
            handler(message_chunk).await
        } else {
            message_chunk
        };
        let _ = sender.send(final_chunk);
    }

    assistant_response
}

/// Store conversation turn in memory
fn store_conversation_in_memory<S: std::hash::BuildHasher>(
    system_prompt: &str,
    user_message: &str,
    assistant_response: &str,
    memory: &Arc<MemoryCoordinator>,
    metadata: &HashMap<String, String, S>,
) {
    // Base metadata template
    let base_meta = MemoryMetadata {
        user_id: metadata.get("user_id").cloned(),
        agent_id: metadata.get("agent_id").cloned(),
        context: "chat".to_string(),
        importance: 0.8,
        keywords: vec![],
        category: "conversation".to_string(),
        source: Some("chat".to_string()),
        created_at: chrono::Utc::now(),
        last_accessed_at: None,
        embedding: None,
        custom: serde_json::Value::Object(serde_json::Map::new()),
        tags: vec![], // Set per message type below
    };

    // Store SYSTEM message
    if !system_prompt.is_empty() {
        let system_meta = MemoryMetadata {
            tags: vec!["message_type.system".to_string()],
            ..base_meta.clone()
        };

        let memory_clone = memory.clone();
        let system_msg = system_prompt.to_string();
        tokio::spawn(async move {
            if let Err(e) = memory_clone
                .add_memory(
                    system_msg,
                    DomainMemoryTypeEnum::Semantic,
                    Some(system_meta),
                )
                .await
            {
                log::error!("Failed to store system memory: {e:?}");
            }
        });
    }

    // Store USER message
    let user_meta = MemoryMetadata {
        tags: vec!["message_type.user".to_string()],
        ..base_meta.clone()
    };

    let memory_clone = memory.clone();
    let user_msg = user_message.to_string();
    tokio::spawn(async move {
        if let Err(e) = memory_clone
            .add_memory(user_msg, DomainMemoryTypeEnum::Episodic, Some(user_meta))
            .await
        {
            log::error!("Failed to store user memory: {e:?}");
        }
    });

    // Store ASSISTANT message
    let assistant_meta = MemoryMetadata {
        tags: vec!["message_type.assistant".to_string()],
        ..base_meta.clone()
    };

    let memory_clone = memory.clone();
    let assistant_msg = assistant_response.to_string();
    tokio::spawn(async move {
        if let Err(e) = memory_clone
            .add_memory(
                assistant_msg,
                DomainMemoryTypeEnum::Episodic,
                Some(assistant_meta),
            )
            .await
        {
            log::error!("Failed to store assistant memory: {e:?}");
        }
    });
}

/// Invoke conversation turn handler if configured
#[allow(clippy::too_many_arguments)]
async fn invoke_turn_handler_if_configured(
    user_message: &str,
    assistant_response: &str,
    sender: &tokio::sync::mpsc::UnboundedSender<CandleMessageChunk>,
    model_config: &CandleModelConfig,
    provider: &TextToTextModel,
    tools: &Arc<[ToolInfo]>,
    on_conversation_turn_handler: Option<&OnConversationTurnHandler>,
) {
    if let Some(handler) = on_conversation_turn_handler {
        let mut conversation = CandleAgentConversation::new();
        conversation.add_message(user_message.to_string(), CandleMessageRole::User);
        conversation.add_message(assistant_response.to_string(), CandleMessageRole::Assistant);

        let builder_state = Arc::new(AgentBuilderState {
            name: String::from("agent"),
            text_to_text_model: provider.clone(),
            text_embedding_model: None,
            temperature: f64::from(model_config.temperature),
            max_tokens: u64::from(model_config.max_tokens.unwrap_or(4096)),
            memory_read_timeout: model_config.timeout_ms,
            system_prompt: model_config.system_prompt.clone().unwrap_or_default(),
            tools: tools.to_vec().into(),
            context_file: None,
            context_files: None,
            context_directory: None,
            context_github: None,
            additional_params: HashMap::new(),
            metadata: HashMap::new(),
            on_chunk_handler: None,
            on_tool_result_handler: None,
            on_conversation_turn_handler: Some(handler.clone()),
        });

        let agent = CandleAgentRoleAgent::new(builder_state);
        let handler_stream: Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>> =
            handler(&conversation, &agent).await;
        tokio::pin!(handler_stream);
        while let Some(chunk) = handler_stream.next().await {
            let _ = sender.send(chunk);
        }
    }
}

/// Execute a tool call and return the result as a message chunk
async fn execute_tool_call(
    name: &str,
    input: &str,
    tool_router: Option<&CandleToolRouter>,
    on_tool_result_handler: Option<&OnToolResultHandler>,
) -> CandleMessageChunk {
    if let Some(router) = tool_router {
        match serde_json::from_str::<serde_json::Value>(input) {
            Ok(args_json) => {
                match router.call_tool(name, args_json).await {
                    Ok(response) => {
                        if let Some(handler) = on_tool_result_handler {
                            let results = vec![format!("{response:?}")];
                            handler(&results).await;
                        }
                        CandleMessageChunk::Text(format!("Tool '{name}' executed: {response:?}"))
                    }
                    Err(e) => CandleMessageChunk::Error(format!("Tool '{name}' failed: {e}")),
                }
            }
            Err(e) => CandleMessageChunk::Error(format!("Invalid JSON: {e}")),
        }
    } else {
        CandleMessageChunk::ToolCallComplete {
            id: String::new(),
            name: name.to_string(),
            input: input.to_string(),
        }
    }
}

/// Handle user prompt/reprompt processing with full conversation flow
#[allow(clippy::too_many_arguments)]
async fn handle_user_prompt<S: std::hash::BuildHasher>(
    user_message: String,
    sender: &tokio::sync::mpsc::UnboundedSender<CandleMessageChunk>,
    chat_config: &CandleChatConfig,
    model_config: &CandleModelConfig,
    provider: &TextToTextModel,
    memory: &Arc<MemoryCoordinator>,
    tools: &Arc<[ToolInfo]>,
    metadata: &HashMap<String, String, S>,
    on_chunk_handler: Option<&OnChunkHandler>,
    on_tool_result_handler: Option<&OnToolResultHandler>,
    on_conversation_turn_handler: Option<&OnConversationTurnHandler>,
) {
    // Validate message length
    if user_message.len() > chat_config.max_message_length {
        let error_chunk = CandleMessageChunk::Error(format!(
            "Message too long: {} characters (max: {})",
            user_message.len(),
            chat_config.max_message_length
        ));
        let _ = sender.send(error_chunk);
        return;
    }

    // Initialize tool router
    let tool_router = initialize_tool_router(sender).await;
    if tool_router.is_none() {
        return; // Error already sent
    }

    // Search memory and build prompt
    let memory_context = search_and_format_memory(memory, &user_message).await;
    let full_prompt =
        build_prompt_with_context(model_config, chat_config, &memory_context, &user_message);

    // Call provider
    let prompt = CandlePrompt::new(full_prompt);
    let mut params = CandleCompletionParams {
        temperature: f64::from(model_config.temperature),
        max_tokens: model_config
            .max_tokens
            .and_then(|t| std::num::NonZeroU64::new(u64::from(t))),
        ..Default::default()
    };

    // Add tools
    if let Some(ref router) = tool_router {
        let mut all_tools: Vec<ToolInfo> = tools.to_vec();
        let auto_generated_tools = router.get_available_tools().await;
        all_tools.extend(auto_generated_tools);

        if !all_tools.is_empty() {
            params.tools = Some(ZeroOneOrMany::from(all_tools));
        }
    }

    // Stream and process completion chunks
    let completion_stream = provider.prompt(prompt, &params);
    let assistant_response = stream_and_process_chunks(
        completion_stream,
        sender,
        chat_config,
        tool_router.as_ref(),
        on_chunk_handler,
        on_tool_result_handler,
    )
    .await;

    // Store conversation in memory including system prompt
    if !assistant_response.is_empty() {
        let system_prompt = build_system_prompt(model_config, chat_config);
        store_conversation_in_memory(
            &system_prompt,
            &user_message,
            &assistant_response,
            memory,
            metadata,
        );
    }

    // Invoke conversation turn handler if configured
    invoke_turn_handler_if_configured(
        &user_message,
        &assistant_response,
        sender,
        model_config,
        provider,
        tools,
        on_conversation_turn_handler,
    )
    .await;
}

pub async fn execute_chat_session<F, Fut, S>(
    config: ChatSessionConfig<S>,
    contexts: ChatSessionContexts,
    conversation_history: ZeroOneOrMany<(CandleMessageRole, String)>,
    handler: F,
    handlers: ChatSessionHandlers,
) -> Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>
where
    F: FnOnce(&CandleAgentConversation) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = CandleChatLoop> + Send + 'static,
    S: std::hash::BuildHasher + Send + Sync + 'static,
{
    Box::pin(crate::async_stream::spawn_stream(
        move |sender| async move {
            // Destructure config and contexts for easier access
            let ChatSessionConfig {
                model_config,
                chat_config,
                provider,
                memory,
                tools,
                metadata,
            } = config;
            let ChatSessionContexts {
                context_file,
                context_files,
                context_directory,
                context_github,
            } = contexts;
            let ChatSessionHandlers {
                on_chunk_handler,
                on_tool_result_handler,
                on_conversation_turn_handler,
            } = handlers;

            // Load context documents from all sources in parallel using tokio::spawn
            let load_tasks = load_all_contexts(
                &memory,
                &metadata,
                context_file,
                context_files,
                context_directory,
                context_github,
            );

            // Wait for all context loading tasks to complete
            for task in load_tasks {
                if let Err(e) = task.await {
                    log::warn!("Context loading task panicked: {e:?}");
                }
            }

            // Create conversation and ALWAYS populate with history (history is not optional)
            let mut initial_conversation = CandleAgentConversation::new();

            // Convert ZeroOneOrMany to vec for iteration
            let history_vec: Vec<(CandleMessageRole, String)> = match conversation_history {
                ZeroOneOrMany::None => vec![],
                ZeroOneOrMany::One(item) => vec![item],
                ZeroOneOrMany::Many(items) => items,
            };

            for (role, message) in history_vec {
                initial_conversation.add_message(message, role);
            }

            // Execute async handler to get CandleChatLoop result
            let chat_loop_result = handler(&initial_conversation).await;

            // Process CandleChatLoop result
            match chat_loop_result {
                CandleChatLoop::Break => {
                    let _ = sender.send(process_break_loop());
                }
                CandleChatLoop::UserPrompt(user_message)
                | CandleChatLoop::Reprompt(user_message) => {
                    handle_user_prompt(
                        user_message,
                        &sender,
                        &chat_config,
                        &model_config,
                        &provider,
                        &memory,
                        &tools,
                        &metadata,
                        on_chunk_handler.as_ref(),
                        on_tool_result_handler.as_ref(),
                        on_conversation_turn_handler.as_ref(),
                    )
                    .await;
                }
            }
        },
    ))
}
