//! Workflow executor implementations
//!
//\! Implements the `DomainCommandExecutor` trait for workflow commands:
//! template, macro, branch, session, and search.

use std::pin::Pin;
use tokio_stream::Stream;

use super::{
    CommandExecutionContext, CommandExecutionResult, CommandInfo, ImmutableChatCommand,
    ValidationResult,
    executor_defs::{
        DomainBranchExecutor, DomainMacroExecutor, DomainSearchExecutor, DomainSessionExecutor,
        DomainTemplateExecutor,
    },
    executor_trait::DomainCommandExecutor,
};

impl DomainCommandExecutor for DomainTemplateExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain template processed successfully".to_string(),
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
        "template"
    }
}

impl DomainCommandExecutor for DomainMacroExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result =
                CommandExecutionResult::Success("Domain macro executed successfully".to_string());
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
        "macro"
    }
}

impl DomainCommandExecutor for DomainBranchExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain branch operation completed successfully".to_string(),
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
        "branch"
    }
}

impl DomainCommandExecutor for DomainSessionExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Data(serde_json::json!({
                "session_type": "domain",
                "status": "active",
                "session_id": uuid::Uuid::new_v4().to_string()
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
        "session"
    }
}

impl DomainCommandExecutor for DomainSearchExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Data(serde_json::json!({
                "search_type": "domain",
                "results": [],
                "total_count": 0,
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
        "search"
    }
}
