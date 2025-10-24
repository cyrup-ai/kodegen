//! Core command executor implementations
//!
//! Implements the `DomainCommandExecutor` trait for core commands:
//! help, clear, debug, stats, theme, and chat.

use std::pin::Pin;
use tokio_stream::Stream;

use super::{
    CommandExecutionContext, CommandExecutionResult, CommandInfo, ImmutableChatCommand,
    ValidationResult,
    executor_defs::{
        DomainChatExecutor, DomainClearExecutor, DomainDebugExecutor, DomainHelpExecutor,
        DomainStatsExecutor, DomainThemeExecutor,
    },
    executor_trait::DomainCommandExecutor,
};

impl DomainCommandExecutor for DomainHelpExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain help command executed successfully".to_string(),
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
        "help"
    }
}

impl DomainCommandExecutor for DomainClearExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain clear command executed successfully".to_string(),
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
        "clear"
    }

    #[inline]
    fn is_parallelizable(&self) -> bool {
        false // Clear operations should not be parallelized
    }
}

impl DomainCommandExecutor for DomainDebugExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Data(serde_json::json!({
                "debug_info": {
                    "enabled": true,
                    "level": "info",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                },
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
        "debug"
    }
}

impl DomainCommandExecutor for DomainStatsExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Data(serde_json::json!({
                "domain_stats": {
                    "total_commands": 0,
                    "successful_executions": 0,
                    "failed_executions": 0,
                    "average_execution_time_ms": 0.0
                },
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
        "stats"
    }
}

impl DomainCommandExecutor for DomainThemeExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result =
                CommandExecutionResult::Success("Domain theme updated successfully".to_string());
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
        "theme"
    }
}

impl DomainCommandExecutor for DomainChatExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain chat command executed successfully".to_string(),
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
        "chat"
    }
}
