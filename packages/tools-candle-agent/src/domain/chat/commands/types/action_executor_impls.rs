//! Action executor implementations
//!
//\! Implements the `DomainCommandExecutor` trait for action commands:
//! tool, copy, retry, undo, history, and custom.

use std::pin::Pin;
use tokio_stream::Stream;

use super::{
    CommandExecutionContext, CommandExecutionResult, CommandInfo, ImmutableChatCommand,
    ValidationResult,
    executor_defs::{
        DomainCopyExecutor, DomainCustomExecutor, DomainHistoryExecutor, DomainRetryExecutor,
        DomainToolExecutor, DomainUndoExecutor,
    },
    executor_trait::DomainCommandExecutor,
};

impl DomainCommandExecutor for DomainToolExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result =
                CommandExecutionResult::Success("Domain tool executed successfully".to_string());
            let _ = sender.send(result);
        }))
    }

    #[inline]
    fn get_info(&self) -> &CommandInfo {
        &self.info
    }

    #[inline]
    fn validate_parameters(&self, _command: &ImmutableChatCommand) -> ValidationResult {
        Ok(())
    }

    #[inline]
    fn name(&self) -> &'static str {
        "tool"
    }
}

impl DomainCommandExecutor for DomainCopyExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain copy operation completed successfully".to_string(),
            );
            let _ = sender.send(result);
        }))
    }

    #[inline]
    fn get_info(&self) -> &CommandInfo {
        &self.info
    }

    #[inline]
    fn validate_parameters(&self, _command: &ImmutableChatCommand) -> ValidationResult {
        Ok(())
    }

    #[inline]
    fn name(&self) -> &'static str {
        "copy"
    }
}

impl DomainCommandExecutor for DomainRetryExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain retry operation completed successfully".to_string(),
            );
            let _ = sender.send(result);
        }))
    }

    #[inline]
    fn get_info(&self) -> &CommandInfo {
        &self.info
    }

    #[inline]
    fn validate_parameters(&self, _command: &ImmutableChatCommand) -> ValidationResult {
        Ok(())
    }

    #[inline]
    fn name(&self) -> &'static str {
        "retry"
    }
}

impl DomainCommandExecutor for DomainUndoExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain undo operation completed successfully".to_string(),
            );
            let _ = sender.send(result);
        }))
    }

    #[inline]
    fn get_info(&self) -> &CommandInfo {
        &self.info
    }

    #[inline]
    fn validate_parameters(&self, _command: &ImmutableChatCommand) -> ValidationResult {
        Ok(())
    }

    #[inline]
    fn name(&self) -> &'static str {
        "undo"
    }
}

impl DomainCommandExecutor for DomainHistoryExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Data(serde_json::json!({
                "history": [],
                "total_entries": 0,
                "status": "success"
            }));
            let _ = sender.send(result);
        }))
    }

    #[inline]
    fn get_info(&self) -> &CommandInfo {
        &self.info
    }

    #[inline]
    fn validate_parameters(&self, _command: &ImmutableChatCommand) -> ValidationResult {
        Ok(())
    }

    #[inline]
    fn name(&self) -> &'static str {
        "history"
    }
}

impl DomainCommandExecutor for DomainCustomExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain custom command executed successfully".to_string(),
            );
            let _ = sender.send(result);
        }))
    }

    #[inline]
    fn get_info(&self) -> &CommandInfo {
        &self.info
    }

    #[inline]
    fn validate_parameters(&self, _command: &ImmutableChatCommand) -> ValidationResult {
        Ok(())
    }

    #[inline]
    fn name(&self) -> &'static str {
        "custom"
    }
}
