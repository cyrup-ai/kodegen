//! Handler registration and wrapping for async callbacks

use super::super::*;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

pub(super) fn wrap_chunk_handler<F, Fut>(handler: F) -> OnChunkHandler
where
    F: Fn(CandleMessageChunk) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = CandleMessageChunk> + Send + 'static,
{
    let wrapped = move |chunk: CandleMessageChunk| {
        Box::pin(handler(chunk))
            as Pin<Box<dyn std::future::Future<Output = CandleMessageChunk> + Send>>
    };
    Arc::new(wrapped)
}

pub(super) fn wrap_tool_result_handler<F, Fut>(handler: F) -> OnToolResultHandler
where
    F: Fn(&[String]) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let wrapped = move |results: &[String]| {
        Box::pin(handler(results)) as Pin<Box<dyn std::future::Future<Output = ()> + Send>>
    };
    Arc::new(wrapped)
}

pub(super) fn wrap_conversation_turn_handler<F, Fut>(handler: F) -> OnConversationTurnHandler
where
    F: Fn(&CandleAgentConversation, &CandleAgentRoleAgent) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>>
        + Send
        + 'static,
{
    let wrapped_handler = move |conv: &CandleAgentConversation, agent: &CandleAgentRoleAgent| {
        Box::pin(handler(conv, agent))
            as Pin<
                Box<
                    dyn std::future::Future<
                            Output = Pin<Box<dyn Stream<Item = CandleMessageChunk> + Send>>,
                        > + Send,
                >,
            >
    };
    Arc::new(wrapped_handler)
}
