//! Trait implementations for command execution events and executor
//!
//! Provides Debug, `MessageChunk`, and Default implementations.

use std::sync::atomic::Ordering;

use cyrup_sugars::prelude::MessageChunk;

use super::super::commands::ImmutableChatCommand;
use super::super::metadata::ResourceUsage;
use super::event_types::CommandEvent;
use super::executor::StreamingCommandExecutor;
use crate::domain::util::unix_timestamp_micros;

impl std::fmt::Debug for StreamingCommandExecutor {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingCommandExecutor")
            .field(
                "execution_counter",
                &self.execution_counter.load(Ordering::Relaxed),
            )
            .field(
                "active_executions",
                &self.active_executions.load(Ordering::Relaxed),
            )
            .field(
                "total_executions",
                &self.total_executions.load(Ordering::Relaxed),
            )
            .field(
                "successful_executions",
                &self.successful_executions.load(Ordering::Relaxed),
            )
            .field(
                "failed_executions",
                &self.failed_executions.load(Ordering::Relaxed),
            )
            .field("event_sender", &self.event_sender.is_some())
            .finish()
    }
}

// MessageChunk implementation for CommandEvent
impl MessageChunk for CommandEvent {
    fn bad_chunk(error: String) -> Self {
        CommandEvent::Failed {
            execution_id: 0,
            error,
            error_code: 500,
            duration_us: 0,
            resource_usage: ResourceUsage::default(),
            timestamp_us: unix_timestamp_micros(),
        }
    }

    fn error(&self) -> Option<&str> {
        match self {
            CommandEvent::Failed { error, .. } => Some(error),
            _ => None,
        }
    }
}

// Default implementation for CommandEvent
impl Default for CommandEvent {
    fn default() -> Self {
        CommandEvent::Started {
            command: ImmutableChatCommand::default(),
            execution_id: 0,
            timestamp_us: unix_timestamp_micros(),
        }
    }
}
