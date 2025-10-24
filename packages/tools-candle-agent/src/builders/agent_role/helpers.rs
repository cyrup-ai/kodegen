//! Helper types and utility functions

use super::*;
use crate::domain::agent::core::AGENT_STATS;
use crate::domain::completion::types::ToolInfo;
use crate::domain::tool::{CandleToolRouter, ToolSelector};
use crate::capability::text_to_text::qwen3_quantized::LoadedQwen3QuantizedModel;
use crate::capability::registry::TextToTextModel;
use kodegen_mcp_client::create_sse_client;

pub struct CandleAgentRoleAgent {
    state: Arc<AgentBuilderState>,
}

impl CandleAgentRoleAgent {
    pub(crate) fn new(state: Arc<AgentBuilderState>) -> Self {
        Self { state }
    }

    /// Chat method for use in on_conversation_turn closure - enables recursion
    pub fn chat(
        &self,
        chat_loop: CandleChatLoop,
    ) -> Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>> {
        match chat_loop {
            CandleChatLoop::Break => {
                Box::pin(crate::async_stream::spawn_stream(|sender| async move {
                    let final_chunk = CandleMessageChunk::Complete {
                        text: String::new(),
                        finish_reason: Some("break".to_string()),
                        usage: None,
                        token_count: None,
                        elapsed_secs: None,
                        tokens_per_sec: None,
                    };
                    let _ = sender.send(final_chunk);
                }))
            }
            CandleChatLoop::UserPrompt(user_message) | CandleChatLoop::Reprompt(user_message) => {
                self.run_inference_cycle(user_message)
            }
        }
    }

    fn run_inference_cycle(
        &self,
        user_message: String,
    ) -> Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>> {
        let state = self.state.clone();

        Box::pin(crate::async_stream::spawn_stream(
            move |stream_sender| async move {
                // Extract handlers from state for recursive inference
                let on_chunk_handler = state.on_chunk_handler.clone();
                let on_tool_result_handler = state.on_tool_result_handler.clone();

                // Connect to kodegen MCP daemon for real tool execution
                let tool_router = match create_sse_client("https://mcp.kodegen.ai:30437/sse").await {
                    Ok((client, _connection)) => {
                        // Connection established - client is cheaply clone-able
                        // _connection dropped here but that's OK - we only need tools for this inference cycle
                        let tool_count = client.list_tools().await
                            .map(|tools| tools.len())
                            .unwrap_or(0);
                        
                        log::info!("✅ Connected to MCP daemon - {} tools available", tool_count);
                        Some(CandleToolRouter::new(Some(client)))
                    }
                    Err(e) => {
                        // Daemon unavailable - agent will work without tools
                        log::warn!("⚠️  MCP daemon unavailable: {} - Agent will work without tools", e);
                        None
                    }
                };

                // Build prompt - system_prompt always exists (no memory in recursive calls)
                let full_prompt = format!("{}\n\nUser: {}", &state.system_prompt, user_message);

                // Call provider
                let prompt = CandlePrompt::new(full_prompt);
                let mut params = crate::domain::completion::CandleCompletionParams {
                    temperature: state.temperature,
                    max_tokens: std::num::NonZeroU64::new(state.max_tokens),
                    ..Default::default()
                };

                // Add tools
                if let Some(ref router) = tool_router {
                    let mut all_tools: Vec<ToolInfo> = state.tools.clone().into();

                    // Use .await instead of block_on
                    let auto_generated_tools = router.get_available_tools().await;
                    all_tools.extend(auto_generated_tools);

                    if !all_tools.is_empty() {
                        // ═══════════════════════════════════════════════════════════
                        // TOOL SELECTION: Filter to 2-3 most relevant tools
                        // ═══════════════════════════════════════════════════════════
                        let final_tools = if all_tools.len() > 3 {
                            // Extract model from enum
                            let TextToTextModel::Qwen3Quantized(base_model) = &state.text_to_text_model;

                            // Load model for tool selection
                            match LoadedQwen3QuantizedModel::load(base_model).await {
                                Ok(loaded_model) => {
                                    let selector = ToolSelector::new(Arc::new(loaded_model));
                                    match selector.select_tools(&user_message, &all_tools).await {
                                        Ok(selected_names) => {
                                            // Filter to selected tools only
                                            all_tools
                                                .into_iter()
                                                .filter(|t| selected_names.iter().any(|n| n.as_str() == t.name.as_ref()))
                                                .collect()
                                        }
                                        Err(e) => {
                                            log::warn!("Tool selection failed: {}, using all tools", e);
                                            all_tools
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::warn!("Failed to load model for tool selection: {}, using all tools", e);
                                    all_tools
                                }
                            }
                        } else {
                            // 3 or fewer tools - no selection needed
                            all_tools
                        };
                        
                        params.tools = Some(ZeroOneOrMany::from(final_tools));
                    }
                }

                let completion_stream = state.text_to_text_model.prompt(prompt, &params);
                tokio::pin!(completion_stream);
                let mut assistant_response = String::new();

                // Stream chunks
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
                                let duration_us =
                                    (elapsed_secs.unwrap_or(0.0) * 1_000_000.0) as u64;
                                AGENT_STATS.record_completion(token_count as u64, duration_us);
                            } else if let Some(usage) = usage {
                                let duration_us =
                                    (elapsed_secs.unwrap_or(0.0) * 1_000_000.0) as u64;
                                AGENT_STATS
                                    .record_completion(usage.total_tokens as u64, duration_us);
                            }

                            CandleMessageChunk::Complete {
                                text: text.clone(),
                                finish_reason: finish_reason.map(|f| format!("{:?}", f)),
                                usage: usage.map(|u| format!("{:?}", u)),
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
                        CandleCompletionChunk::ToolCallComplete { id, name, input } => {
                            if let Some(ref router) = tool_router {
                                match serde_json::from_str::<serde_json::Value>(&input) {
                                    Ok(args_json) => {
                                        // Use .await instead of block_on
                                        match router.call_tool(&name, args_json).await {
                                            Ok(response) => {
                                                // Call tool result handler if configured
                                                if let Some(ref handler) = on_tool_result_handler {
                                                    let results = vec![format!("{:?}", response)];
                                                    handler(&results).await;
                                                }

                                                CandleMessageChunk::Text(format!(
                                                    "Tool '{}' executed: {:?}",
                                                    name, response
                                                ))
                                            }
                                            Err(e) => CandleMessageChunk::Error(format!(
                                                "Tool '{}' failed: {}",
                                                name, e
                                            )),
                                        }
                                    }
                                    Err(e) => {
                                        CandleMessageChunk::Error(format!("Invalid JSON: {}", e))
                                    }
                                }
                            } else {
                                CandleMessageChunk::ToolCallComplete { id, name, input }
                            }
                        }
                        CandleCompletionChunk::Error(error) => CandleMessageChunk::Error(error),
                    };

                    // Apply chunk handler if configured (zero allocation for None)
                    let final_chunk = if let Some(ref handler) = on_chunk_handler {
                        handler(message_chunk).await
                    } else {
                        message_chunk
                    };
                    let _ = stream_sender.send(final_chunk);
                }

                // CRITICAL: Call handler for recursion
                if let Some(ref handler) = state.on_conversation_turn_handler {
                    let mut conversation = CandleAgentConversation::new();
                    conversation.add_message(user_message.clone(), CandleMessageRole::User);
                    conversation
                        .add_message(assistant_response.clone(), CandleMessageRole::Assistant);

                    let agent = CandleAgentRoleAgent {
                        state: state.clone(),
                    };
                    // Await the async handler to get the stream
                    let handler_stream = handler(&conversation, &agent).await;
                    tokio::pin!(handler_stream);
                    while let Some(chunk) = handler_stream.next().await {
                        let _ = stream_sender.send(chunk);
                    }
                }
            },
        ))
    }
}

pub trait ConversationHistoryArgs {
    /// Convert this into conversation history format
    fn into_history(self) -> ZeroOneOrMany<(CandleMessageRole, String)>;
}

impl ConversationHistoryArgs for (CandleMessageRole, &str) {
    fn into_history(self) -> ZeroOneOrMany<(CandleMessageRole, String)> {
        ZeroOneOrMany::one((self.0, self.1.to_string()))
    }
}

impl ConversationHistoryArgs for (CandleMessageRole, String) {
    fn into_history(self) -> ZeroOneOrMany<(CandleMessageRole, String)> {
        ZeroOneOrMany::one(self)
    }
}

impl<T1, T2> ConversationHistoryArgs for (T1, T2)
where
    T1: ConversationHistoryArgs,
    T2: ConversationHistoryArgs,
{
    fn into_history(self) -> ZeroOneOrMany<(CandleMessageRole, String)> {
        let h1 = self.0.into_history();
        let h2 = self.1.into_history();

        match (h1, h2) {
            (ZeroOneOrMany::None, h) | (h, ZeroOneOrMany::None) => h,
            (ZeroOneOrMany::One(m1), ZeroOneOrMany::One(m2)) => ZeroOneOrMany::Many(vec![m1, m2]),
            (ZeroOneOrMany::One(m), ZeroOneOrMany::Many(mut msgs)) => {
                msgs.insert(0, m);
                ZeroOneOrMany::Many(msgs)
            }
            (ZeroOneOrMany::Many(mut msgs), ZeroOneOrMany::One(m)) => {
                msgs.push(m);
                ZeroOneOrMany::Many(msgs)
            }
            (ZeroOneOrMany::Many(mut msgs1), ZeroOneOrMany::Many(msgs2)) => {
                msgs1.extend(msgs2);
                ZeroOneOrMany::Many(msgs1)
            }
        }
    }
}

impl<T1, T2, T3> ConversationHistoryArgs for (T1, T2, T3)
where
    T1: ConversationHistoryArgs,
    T2: ConversationHistoryArgs,
    T3: ConversationHistoryArgs,
{
    fn into_history(self) -> ZeroOneOrMany<(CandleMessageRole, String)> {
        let h1 = self.0.into_history();
        let h2 = self.1.into_history();
        let h3 = self.2.into_history();

        // Merge all three by first merging h1 and h2, then merging result with h3
        let combined_12 = match (h1, h2) {
            (ZeroOneOrMany::None, h) | (h, ZeroOneOrMany::None) => h,
            (ZeroOneOrMany::One(m1), ZeroOneOrMany::One(m2)) => ZeroOneOrMany::Many(vec![m1, m2]),
            (ZeroOneOrMany::One(m), ZeroOneOrMany::Many(mut msgs)) => {
                msgs.insert(0, m);
                ZeroOneOrMany::Many(msgs)
            }
            (ZeroOneOrMany::Many(mut msgs), ZeroOneOrMany::One(m)) => {
                msgs.push(m);
                ZeroOneOrMany::Many(msgs)
            }
            (ZeroOneOrMany::Many(mut msgs1), ZeroOneOrMany::Many(msgs2)) => {
                msgs1.extend(msgs2);
                ZeroOneOrMany::Many(msgs1)
            }
        };

        // Now merge combined_12 with h3
        match (combined_12, h3) {
            (ZeroOneOrMany::None, h) | (h, ZeroOneOrMany::None) => h,
            (ZeroOneOrMany::One(m1), ZeroOneOrMany::One(m2)) => ZeroOneOrMany::Many(vec![m1, m2]),
            (ZeroOneOrMany::One(m), ZeroOneOrMany::Many(mut msgs)) => {
                msgs.insert(0, m);
                ZeroOneOrMany::Many(msgs)
            }
            (ZeroOneOrMany::Many(mut msgs), ZeroOneOrMany::One(m)) => {
                msgs.push(m);
                ZeroOneOrMany::Many(msgs)
            }
            (ZeroOneOrMany::Many(mut msgs1), ZeroOneOrMany::Many(msgs2)) => {
                msgs1.extend(msgs2);
                ZeroOneOrMany::Many(msgs1)
            }
        }
    }
}

/// CandleFluentAi entry point for creating agent roles
pub struct CandleFluentAi;

impl CandleFluentAi {
    /// Create a new agent role builder - main entry point
    pub fn agent_role(name: impl Into<String>) -> impl CandleAgentRoleBuilder {
        CandleAgentRoleBuilderImpl::new(name)
    }
    
    /// Create a new vision builder - entry point for vision operations
    pub fn vision() -> impl crate::builders::vision::CandleVisionBuilder {
        crate::builders::vision::VisionBuilderImpl::new()
    }
}
