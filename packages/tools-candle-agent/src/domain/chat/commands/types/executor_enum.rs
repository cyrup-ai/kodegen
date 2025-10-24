//! Enum-based zero-allocation executor dispatch
//!
//! Provides enum dispatch instead of trait objects to eliminate boxing
//! and virtual calls for maximum performance.

use std::pin::Pin;
use tokio_stream::Stream;

use super::{
    CommandExecutionContext, CommandExecutionResult, CommandInfo, ImmutableChatCommand,
    ValidationResult,
    executor_defs::{
        DomainBranchExecutor, DomainChatExecutor, DomainClearExecutor, DomainConfigExecutor,
        DomainCopyExecutor, DomainCustomExecutor, DomainDebugExecutor, DomainExportExecutor,
        DomainHelpExecutor, DomainHistoryExecutor, DomainImportExecutor, DomainLoadExecutor,
        DomainMacroExecutor, DomainRetryExecutor, DomainSaveExecutor, DomainSearchExecutor,
        DomainSessionExecutor, DomainSettingsExecutor, DomainStatsExecutor, DomainTemplateExecutor,
        DomainThemeExecutor, DomainToolExecutor, DomainUndoExecutor,
    },
    executor_trait::DomainCommandExecutor,
};

/// Domain command executor enum for zero-allocation dispatch
/// Uses enum dispatch instead of trait objects to eliminate boxing and virtual calls
#[derive(Debug, Clone)]
pub enum DomainCommandExecutorEnum {
    Help(DomainHelpExecutor),
    Clear(DomainClearExecutor),
    Export(DomainExportExecutor),
    Config(DomainConfigExecutor),
    Template(DomainTemplateExecutor),
    Macro(DomainMacroExecutor),
    Search(DomainSearchExecutor),
    Branch(DomainBranchExecutor),
    Session(DomainSessionExecutor),
    Tool(DomainToolExecutor),
    Stats(DomainStatsExecutor),
    Theme(DomainThemeExecutor),
    Debug(DomainDebugExecutor),
    History(DomainHistoryExecutor),
    Save(DomainSaveExecutor),
    Load(DomainLoadExecutor),
    Import(DomainImportExecutor),
    Settings(DomainSettingsExecutor),
    Custom(DomainCustomExecutor),
    Copy(DomainCopyExecutor),
    Retry(DomainRetryExecutor),
    Undo(DomainUndoExecutor),
    Chat(DomainChatExecutor),
}

impl DomainCommandExecutorEnum {
    /// Execute command using enum dispatch for zero allocation and maximum performance
    #[inline]
    pub fn execute(
        &self,
        context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>> {
        match self {
            Self::Help(executor) => executor.execute(context),
            Self::Clear(executor) => executor.execute(context),
            Self::Export(executor) => executor.execute(context),
            Self::Config(executor) => executor.execute(context),
            Self::Template(executor) => executor.execute(context),
            Self::Macro(executor) => executor.execute(context),
            Self::Search(executor) => executor.execute(context),
            Self::Branch(executor) => executor.execute(context),
            Self::Session(executor) => executor.execute(context),
            Self::Tool(executor) => executor.execute(context),
            Self::Stats(executor) => executor.execute(context),
            Self::Theme(executor) => executor.execute(context),
            Self::Debug(executor) => executor.execute(context),
            Self::History(executor) => executor.execute(context),
            Self::Save(executor) => executor.execute(context),
            Self::Load(executor) => executor.execute(context),
            Self::Import(executor) => executor.execute(context),
            Self::Settings(executor) => executor.execute(context),
            Self::Custom(executor) => executor.execute(context),
            Self::Copy(executor) => executor.execute(context),
            Self::Retry(executor) => executor.execute(context),
            Self::Undo(executor) => executor.execute(context),
            Self::Chat(executor) => executor.execute(context),
        }
    }

    /// Get command info using enum dispatch - zero allocation
    #[inline]
    #[must_use]
    pub fn get_info(&self) -> &CommandInfo {
        match self {
            Self::Help(executor) => executor.get_info(),
            Self::Clear(executor) => executor.get_info(),
            Self::Export(executor) => executor.get_info(),
            Self::Config(executor) => executor.get_info(),
            Self::Template(executor) => executor.get_info(),
            Self::Macro(executor) => executor.get_info(),
            Self::Search(executor) => executor.get_info(),
            Self::Branch(executor) => executor.get_info(),
            Self::Session(executor) => executor.get_info(),
            Self::Tool(executor) => executor.get_info(),
            Self::Stats(executor) => executor.get_info(),
            Self::Theme(executor) => executor.get_info(),
            Self::Debug(executor) => executor.get_info(),
            Self::History(executor) => executor.get_info(),
            Self::Save(executor) => executor.get_info(),
            Self::Load(executor) => executor.get_info(),
            Self::Import(executor) => executor.get_info(),
            Self::Settings(executor) => executor.get_info(),
            Self::Custom(executor) => executor.get_info(),
            Self::Copy(executor) => executor.get_info(),
            Self::Retry(executor) => executor.get_info(),
            Self::Undo(executor) => executor.get_info(),
            Self::Chat(executor) => executor.get_info(),
        }
    }

    /// Get command name using enum dispatch - zero allocation
    #[inline]
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Help(_) => "help",
            Self::Clear(_) => "clear",
            Self::Export(_) => "export",
            Self::Config(_) => "config",
            Self::Template(_) => "template",
            Self::Macro(_) => "macro",
            Self::Search(_) => "search",
            Self::Branch(_) => "branch",
            Self::Session(_) => "session",
            Self::Tool(_) => "tool",
            Self::Stats(_) => "stats",
            Self::Theme(_) => "theme",
            Self::Debug(_) => "debug",
            Self::History(_) => "history",
            Self::Save(_) => "save",
            Self::Load(_) => "load",
            Self::Import(_) => "import",
            Self::Settings(_) => "settings",
            Self::Custom(_) => "custom",
            Self::Copy(_) => "copy",
            Self::Retry(_) => "retry",
            Self::Undo(_) => "undo",
            Self::Chat(_) => "chat",
        }
    }

    /// Validate parameters using enum dispatch
    ///
    /// # Errors
    /// Returns `ValidationError` if the command parameters fail validation for the specific command type.
    #[inline]
    pub fn validate_parameters(&self, command: &ImmutableChatCommand) -> ValidationResult {
        match self {
            Self::Help(executor) => executor.validate_parameters(command),
            Self::Clear(executor) => executor.validate_parameters(command),
            Self::Export(executor) => executor.validate_parameters(command),
            Self::Config(executor) => executor.validate_parameters(command),
            Self::Template(executor) => executor.validate_parameters(command),
            Self::Macro(executor) => executor.validate_parameters(command),
            Self::Search(executor) => executor.validate_parameters(command),
            Self::Branch(executor) => executor.validate_parameters(command),
            Self::Session(executor) => executor.validate_parameters(command),
            Self::Tool(executor) => executor.validate_parameters(command),
            Self::Stats(executor) => executor.validate_parameters(command),
            Self::Theme(executor) => executor.validate_parameters(command),
            Self::Debug(executor) => executor.validate_parameters(command),
            Self::History(executor) => executor.validate_parameters(command),
            Self::Save(executor) => executor.validate_parameters(command),
            Self::Load(executor) => executor.validate_parameters(command),
            Self::Import(executor) => executor.validate_parameters(command),
            Self::Settings(executor) => executor.validate_parameters(command),
            Self::Custom(executor) => executor.validate_parameters(command),
            Self::Copy(executor) => executor.validate_parameters(command),
            Self::Retry(executor) => executor.validate_parameters(command),
            Self::Undo(executor) => executor.validate_parameters(command),
            Self::Chat(executor) => executor.validate_parameters(command),
        }
    }
}
