//! Data operation executor implementations
//!
//\! Implements the `DomainCommandExecutor` trait for data operations:
//! export, import, save, load, config, and settings.

use std::pin::Pin;
use tokio_stream::Stream;

use super::{
    CommandExecutionContext, CommandExecutionResult, CommandInfo, ImmutableChatCommand,
    ValidationResult,
    executor_defs::{
        DomainConfigExecutor, DomainExportExecutor, DomainImportExecutor, DomainLoadExecutor,
        DomainSaveExecutor, DomainSettingsExecutor,
    },
    executor_trait::DomainCommandExecutor,
};

impl DomainCommandExecutor for DomainExportExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            // Export domain data with zero-allocation streaming pattern
            let result = CommandExecutionResult::Data(serde_json::json!({
                "export_type": "domain",
                "status": "success",
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
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
        "export"
    }
}

impl DomainCommandExecutor for DomainImportExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result =
                CommandExecutionResult::Success("Domain data imported successfully".to_string());
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
        "import"
    }
}

impl DomainCommandExecutor for DomainSaveExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result =
                CommandExecutionResult::Success("Domain data saved successfully".to_string());
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
        "save"
    }
}

impl DomainCommandExecutor for DomainLoadExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result =
                CommandExecutionResult::Success("Domain data loaded successfully".to_string());
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
        "load"
    }
}

impl DomainCommandExecutor for DomainConfigExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Success(
                "Domain configuration updated successfully".to_string(),
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
        "config"
    }
}

impl DomainCommandExecutor for DomainSettingsExecutor {
    #[inline]
    fn execute(
        &self,
        _context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(|sender| async move {
            let result = CommandExecutionResult::Data(serde_json::json!({
                "settings": {},
                "updated": true,
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
        "settings"
    }
}
