//! Candle chat command system with zero-allocation patterns
//!
//! This module provides a comprehensive command system for chat interactions including
//! slash commands, command parsing, execution, and auto-completion with zero-allocation
//! patterns and blazing-fast performance.

pub mod execution;
pub mod parsing;
pub mod registry;
pub mod response;
pub mod types;
pub mod validation;

// Re-export main Candle types and functions for convenience
// Global Candle command executor functionality
use crate::domain::util::unix_timestamp_micros;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use execution::CommandExecutor;

pub use parsing::{CommandParser, ParseError, ParseResult};
pub use registry::CommandRegistry;
pub use response::ResponseFormatter;
pub use types::*;
pub use validation::CommandValidator;

use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

/// Global Candle command executor instance
static CANDLE_COMMAND_EXECUTOR: std::sync::LazyLock<Arc<RwLock<Option<CommandExecutor>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Get global Candle command executor
pub async fn get_candle_command_executor() -> Option<CommandExecutor> {
    let guard = CANDLE_COMMAND_EXECUTOR.read().await;
    guard.clone()
}

/// Parse Candle command using global executor
///
/// # Errors
///
/// Returns `CandleCommandError` if:
/// - Candle command executor is not initialized
/// - Command parsing fails
pub async fn parse_candle_command(input: &str) -> CommandResult<ImmutableChatCommand> {
    if let Some(executor) = get_candle_command_executor().await {
        executor
            .parser()
            .parse(input)
            .map_err(|e| CandleCommandError::ParseError(e.to_string()))
    } else {
        Err(CandleCommandError::ConfigurationError {
            detail: "Candle command executor not initialized".to_string(),
        })
    }
}

/// Execute Candle command using global executor - STREAMING VERSION (streams-only architecture)
#[must_use]
pub fn execute_candle_command_async(
    command: ImmutableChatCommand,
) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
    Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
        if let Some(executor) = get_candle_command_executor().await {
            let mut result_stream = executor.execute_streaming(1, command);
            // Forward all events from executor stream
            while let Some(event) = result_stream.next().await {
                let _ = tx.send(event);
            }
        } else {
            let _ = tx.send(CommandEvent::Failed {
                execution_id: 0,
                error: "Candle command executor not initialized".to_string(),
                error_code: 1001,
                duration_us: 0,
                resource_usage: crate::domain::chat::commands::types::metadata::ResourceUsage::new_with_start_time(),
                timestamp_us: unix_timestamp_micros()
            });
        }
    }))
}

/// Execute Candle command using global executor - STREAMING VERSION (streams-only architecture)
/// Note: Sync version removed - use streaming architecture only
#[must_use]
pub fn execute_candle_command(
    command: ImmutableChatCommand,
) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
    Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
        if let Some(executor) = get_candle_command_executor().await {
            let mut result_stream = executor.execute_streaming(1, command);
            // Forward all events from executor stream
            while let Some(event) = result_stream.next().await {
                let _ = tx.send(event);
            }
        } else {
            let _ = tx.send(CommandEvent::Failed {
                execution_id: 0,
                error: "Candle command executor not initialized".to_string(),
                error_code: 1001,
                duration_us: 0,
                resource_usage: crate::domain::chat::commands::types::metadata::ResourceUsage::new_with_start_time(),
                timestamp_us: unix_timestamp_micros()
            });
        }
    }))
}

/// Parse and execute Candle command using global executor - STREAMING VERSION (streams-only architecture)
#[must_use]
pub fn parse_and_execute_candle_command_async(
    input: &str,
) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
    let input_str = input.to_string();
    Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
        if let Some(executor) = get_candle_command_executor().await {
            let mut result_stream = executor.parse_and_execute(&input_str);
            // Forward all events from executor stream
            while let Some(event) = result_stream.next().await {
                let _ = tx.send(event);
            }
        } else {
            let _ = tx.send(CommandEvent::Failed {
                execution_id: 0,
                error: "Candle command executor not initialized".to_string(),
                error_code: 1001,
                duration_us: 0,
                resource_usage: crate::domain::chat::commands::types::metadata::ResourceUsage::new_with_start_time(),
                timestamp_us: unix_timestamp_micros()
            });
        }
    }))
}

/// Parse and execute Candle command using global executor - STREAMING VERSION (streams-only architecture)
/// Note: Sync version removed - use streaming architecture only
#[must_use]
pub fn parse_and_execute_candle_command(
    input: &str,
) -> Pin<Box<dyn Stream<Item = CommandEvent> + Send>> {
    let input_str = input.to_string();
    Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
        if let Some(executor) = get_candle_command_executor().await {
            let mut result_stream = executor.parse_and_execute(&input_str);
            // Forward all events from executor stream
            while let Some(event) = result_stream.next().await {
                let _ = tx.send(event);
            }
        } else {
            let _ = tx.send(CommandEvent::Failed {
                execution_id: 0,
                error: "Candle command executor not initialized".to_string(),
                error_code: 1001,
                duration_us: 0,
                resource_usage: crate::domain::chat::commands::types::metadata::ResourceUsage::new_with_start_time(),
                timestamp_us: unix_timestamp_micros()
            });
        }
    }))
}
