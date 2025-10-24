//! Main command validator with dispatch logic
//!
//\! Provides the `CommandValidator` struct that coordinates validation
//! across all command types and parameters.

use super::super::types::ImmutableChatCommand;
use super::errors::ValidationError;
use super::parameter_validators::ValidationConfig;
use super::security::get_security_patterns;
use regex::Regex;

/// Command validator with comprehensive validation rules
#[derive(Debug, Clone)]
pub struct CommandValidator {
    config: ValidationConfig,
    blocked_patterns: Vec<Regex>,
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandValidator {
    /// Create a new command validator with default settings
    ///
    /// Uses pre-compiled static regexes for security validation (compiled once on first access).
    /// Cloning these regexes is cheap due to internal reference counting.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
            blocked_patterns: get_security_patterns(),
        }
    }

    /// Validate a command with comprehensive checks
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if any command parameter fails validation checks
    /// including string length, integer ranges, file paths, or command-specific rules
    #[inline]
    pub fn validate_command(&self, command: &ImmutableChatCommand) -> Result<(), ValidationError> {
        match command {
            ImmutableChatCommand::Help { command, .. } => {
                self.config.validate_help_command(command.as_ref())
            }
            ImmutableChatCommand::Clear { keep_last, .. } => {
                ValidationConfig::validate_clear_command(keep_last.as_ref())
            }
            ImmutableChatCommand::Export { format, output, .. } => {
                self.config.validate_export_command(format, output.as_ref())
            }
            ImmutableChatCommand::Config { key, value, .. } => self
                .config
                .validate_config_command(key.as_deref(), value.as_deref()),
            ImmutableChatCommand::Search { query, limit, .. } => {
                self.config.validate_search_command(query, *limit)
            }
            ImmutableChatCommand::Template {
                name,
                content,
                variables,
                ..
            } => self.config.validate_template_command(
                name.as_deref(),
                content.as_deref(),
                variables,
            ),
            ImmutableChatCommand::Macro { name, commands, .. } => self
                .config
                .validate_macro_command(name.as_deref(), commands),
            ImmutableChatCommand::Branch { name, source, .. } => self
                .config
                .validate_branch_command(name.as_deref(), source.as_deref()),
            ImmutableChatCommand::Session { name, .. } => {
                self.config.validate_session_command(name.as_deref())
            }
            ImmutableChatCommand::Tool { name, args, .. } => {
                self.config.validate_tool_command(name.as_deref(), args)
            }
            ImmutableChatCommand::Stats { period, .. } => {
                ValidationConfig::validate_stats_command(period.as_deref())
            }
            ImmutableChatCommand::Theme {
                name, properties, ..
            } => self
                .config
                .validate_theme_command(name.as_deref(), properties),
            ImmutableChatCommand::Debug { level, .. } => {
                ValidationConfig::validate_debug_command(level.as_deref())
            }
            ImmutableChatCommand::History { filter, .. } => {
                self.config.validate_history_command(filter.as_deref())
            }
            ImmutableChatCommand::Save { name, location, .. } => self
                .config
                .validate_save_command(name.as_deref(), location.as_deref()),
            ImmutableChatCommand::Load { name, location, .. } => {
                self.config.validate_load_command(name, location.as_deref())
            }
            ImmutableChatCommand::Import { source, .. } => {
                self.config.validate_import_command(source)
            }
            ImmutableChatCommand::Settings { key, value, .. } => self
                .config
                .validate_settings_command(key.as_deref(), value.as_deref()),
            ImmutableChatCommand::Custom { name, args, .. } => {
                self.config.validate_custom_command(name, args)
            }
            ImmutableChatCommand::Copy {
                message_id,
                content,
                ..
            } => self
                .config
                .validate_copy_command(message_id.as_deref(), content.as_deref()),
            ImmutableChatCommand::Retry {
                command, attempts, ..
            } => self
                .config
                .validate_retry_command(command.as_deref(), attempts.map(|a| a as usize)),
            ImmutableChatCommand::Undo { count, .. } => {
                ValidationConfig::validate_undo_command(*count)
            }
            ImmutableChatCommand::Chat {
                message,
                context,
                priority,
            } => self
                .config
                .validate_chat_command(message, context.as_deref(), Some(*priority)),
        }
    }

    /// Sanitize input string
    #[must_use]
    pub fn sanitize_input(&self, input: &str) -> String {
        // Remove null bytes
        let sanitized = input.replace('\0', "");

        // Limit length
        if sanitized.len() > self.config.max_command_length {
            sanitized[..self.config.max_command_length].to_string()
        } else {
            sanitized
        }
    }

    /// Check if input is safe
    #[must_use]
    pub fn is_safe_input(&self, input: &str) -> bool {
        // Check length
        if input.len() > self.config.max_command_length {
            return false;
        }

        // Check for blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(input) {
                return false;
            }
        }

        true
    }
}
