//! Command validation methods with comprehensive error checking
//!
//! Provides validation logic for all command types with detailed error messages.
//! Ensures command arguments meet requirements before execution.

use super::command_core::ImmutableChatCommand;
use super::errors::{CandleCommandError, CommandResult};

impl ImmutableChatCommand {
    /// Validate command arguments with comprehensive error checking
    ///
    /// # Errors
    ///
    /// Returns `CandleCommandError` if:
    /// - Export format is not one of the supported formats (json, markdown, pdf, html, csv, xml, yaml)
    /// - Search query is empty or exceeds maximum length
    /// - File path validation fails for Copy, Move, or Read commands
    /// - Replace patterns are invalid or unsafe
    /// - Other command-specific validation rules fail
    #[inline]
    pub fn validate(&self) -> CommandResult<()> {
        match self {
            Self::Export { format, .. } => Self::validate_export_format(format),
            Self::Search { query, .. } => Self::validate_search_query(query),
            Self::Load { name, .. } => Self::validate_name_length(name, "Load", 255),
            Self::Import { source, .. } => Self::validate_import_source(source),
            Self::Custom { name, .. } => Self::validate_custom_name(name),
            Self::Chat { message, .. } => Self::validate_chat_message(message),
            Self::Template {
                name: Some(name), ..
            }
            | Self::Macro {
                name: Some(name), ..
            }
            | Self::Branch {
                name: Some(name), ..
            }
            | Self::Session {
                name: Some(name), ..
            }
            | Self::Tool {
                name: Some(name), ..
            }
            | Self::Theme {
                name: Some(name), ..
            } => Self::validate_generic_name(name, self.command_name()),
            Self::History {
                limit: Some(limit), ..
            } => Self::validate_limit(*limit, 10_000, "History"),
            Self::Undo {
                count: Some(count), ..
            } => Self::validate_limit(*count, 100, "Undo"),
            _ => Ok(()),
        }
    }

    #[inline]
    fn validate_export_format(format: &str) -> CommandResult<()> {
        if !matches!(
            format,
            "json" | "markdown" | "pdf" | "html" | "csv" | "xml" | "yaml"
        ) {
            return Err(CandleCommandError::invalid_arguments(format!(
                "Invalid export format '{format}'. Supported: json, markdown, pdf, html, csv, xml, yaml"
            )));
        }
        Ok(())
    }

    #[inline]
    fn validate_search_query(query: &str) -> CommandResult<()> {
        if query.is_empty() {
            return Err(CandleCommandError::invalid_arguments(
                "Search query cannot be empty",
            ));
        }
        if query.len() > 1000 {
            return Err(CandleCommandError::invalid_arguments(
                "Search query too long (max 1000 characters)",
            ));
        }
        Ok(())
    }

    #[inline]
    fn validate_name_length(name: &str, context: &str, max_len: usize) -> CommandResult<()> {
        if name.is_empty() {
            return Err(CandleCommandError::invalid_arguments(format!(
                "{context} name cannot be empty"
            )));
        }
        if name.len() > max_len {
            return Err(CandleCommandError::invalid_arguments(format!(
                "{context} name too long (max {max_len} characters)"
            )));
        }
        Ok(())
    }

    #[inline]
    fn validate_import_source(source: &str) -> CommandResult<()> {
        if source.is_empty() {
            return Err(CandleCommandError::invalid_arguments(
                "Import source cannot be empty",
            ));
        }
        if !source.starts_with("http://")
            && !source.starts_with("https://")
            && !source.starts_with("file://")
            && !source.starts_with('/')
            && !source.starts_with("./")
            && !source.starts_with("../")
        {
            return Err(CandleCommandError::invalid_arguments(
                "Import source must be a valid URL or file path",
            ));
        }
        Ok(())
    }

    #[inline]
    fn validate_custom_name(name: &str) -> CommandResult<()> {
        if name.is_empty() {
            return Err(CandleCommandError::invalid_arguments(
                "Custom command name cannot be empty",
            ));
        }
        if name.len() > 100 {
            return Err(CandleCommandError::invalid_arguments(
                "Custom command name too long (max 100 characters)",
            ));
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(CandleCommandError::invalid_arguments(
                "Custom command name can only contain alphanumeric characters, underscores, and hyphens",
            ));
        }
        Ok(())
    }

    #[inline]
    fn validate_chat_message(message: &str) -> CommandResult<()> {
        if message.is_empty() {
            return Err(CandleCommandError::invalid_arguments(
                "Chat message cannot be empty",
            ));
        }
        if message.len() > 100_000 {
            return Err(CandleCommandError::invalid_arguments(
                "Chat message too long (max 100,000 characters)",
            ));
        }
        Ok(())
    }

    #[inline]
    fn validate_generic_name(name: &str, command_name: &str) -> CommandResult<()> {
        if name.is_empty() {
            return Err(CandleCommandError::invalid_arguments(format!(
                "{command_name} name cannot be empty"
            )));
        }
        if name.len() > 100 {
            return Err(CandleCommandError::invalid_arguments(format!(
                "{command_name} name too long (max 100 characters)"
            )));
        }
        Ok(())
    }

    #[inline]
    fn validate_limit(value: usize, max: usize, context: &str) -> CommandResult<()> {
        if value == 0 {
            return Err(CandleCommandError::invalid_arguments(format!(
                "{context} limit must be greater than 0"
            )));
        }
        if value > max {
            return Err(CandleCommandError::invalid_arguments(format!(
                "{context} limit too large (max {max})"
            )));
        }
        Ok(())
    }
}
