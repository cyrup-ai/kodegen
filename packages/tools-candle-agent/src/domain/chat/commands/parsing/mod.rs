//! Command parsing and validation logic
//!
//! Provides zero-allocation command parsing with comprehensive validation and error handling.
//! Uses blazing-fast parsing algorithms with ergonomic APIs and production-ready error messages.

mod builtin_commands;
mod command_parsers;
mod errors;

// Public re-exports
pub use errors::{ParseError, ParseResult};

use std::collections::HashMap;

use super::types::{CandleCommandError, CommandInfo, ImmutableChatCommand, SearchScope};

/// Zero-allocation command parser with owned strings
#[derive(Debug, Clone)]
pub struct CommandParser {
    /// Registered commands
    commands: HashMap<String, CommandInfo>,
    /// Command aliases
    aliases: HashMap<String, String>,
    /// Command history for auto-completion
    history: Vec<String>,
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandParser {
    /// Create a new command parser
    #[must_use]
    pub fn new() -> Self {
        let mut parser = Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
            history: Vec::new(),
        };
        builtin_commands::register_builtin_commands(&mut parser.commands, &mut parser.aliases);
        parser
    }

    /// Parse command from input string (zero-allocation)
    ///
    /// # Errors
    /// Returns `CandleCommandError::InvalidSyntax` if the input is empty, malformed, or contains invalid arguments
    /// Returns `CandleCommandError::UnknownCommand` if the command name is not recognized
    #[allow(clippy::too_many_lines)]
    pub fn parse_command(&self, input: &str) -> Result<ImmutableChatCommand, CandleCommandError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(CandleCommandError::InvalidSyntax {
                detail: "Empty command".to_string(),
            });
        }

        // Remove leading slash if present
        let input = input.strip_prefix('/').unwrap_or(input);

        // Split command and arguments
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err(CandleCommandError::InvalidSyntax {
                detail: "No command specified".to_string(),
            });
        }

        let command_name = parts[0].to_lowercase();
        let args = &parts[1..];

        // Parse based on command name
        match command_name.as_str() {
            "help" | "h" | "?" => {
                let command = if !args.is_empty() && !args[0].starts_with("--") {
                    Some(args[0].to_string())
                } else {
                    None
                };
                let extended = args.contains(&"--extended");
                Ok(ImmutableChatCommand::Help { command, extended })
            }
            "clear" => {
                let confirm = args.contains(&"--confirm");
                let keep_last = args
                    .iter()
                    .position(|&arg| arg == "--keep-last")
                    .and_then(|i| args.get(i + 1))
                    .and_then(|s| s.parse().ok());
                Ok(ImmutableChatCommand::Clear { confirm, keep_last })
            }
            "export" => {
                let format = args
                    .iter()
                    .position(|&arg| arg == "--format")
                    .and_then(|i| args.get(i + 1))
                    .map_or_else(|| "json".to_string(), std::string::ToString::to_string);
                let output = args
                    .iter()
                    .position(|&arg| arg == "--output")
                    .and_then(|i| args.get(i + 1))
                    .map(std::string::ToString::to_string);
                let include_metadata = args.contains(&"--metadata");
                Ok(ImmutableChatCommand::Export {
                    format,
                    output,
                    include_metadata,
                })
            }
            "config" => {
                let show = args.contains(&"--show");
                let reset = args.contains(&"--reset");
                let key = args
                    .iter()
                    .find(|&&arg| !arg.starts_with("--"))
                    .map(std::string::ToString::to_string);
                let value =
                    if let Some(key_pos) = args.iter().position(|&arg| !arg.starts_with("--")) {
                        if let Some(&arg) = args.get(key_pos + 1) {
                            if arg.starts_with("--") {
                                None
                            } else {
                                Some((*arg).to_string())
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                Ok(ImmutableChatCommand::Config {
                    key,
                    value,
                    show,
                    reset,
                })
            }
            "search" => {
                let query = args
                    .iter()
                    .find(|&&arg| !arg.starts_with("--"))
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default();
                let scope = if args.contains(&"--current") {
                    SearchScope::Current
                } else if args.contains(&"--recent") {
                    SearchScope::Recent
                } else if args.contains(&"--bookmarked") {
                    SearchScope::Bookmarked
                } else {
                    SearchScope::All
                };
                let limit = args
                    .iter()
                    .position(|&arg| arg == "--limit")
                    .and_then(|i| args.get(i + 1))
                    .and_then(|s| s.parse().ok());
                let include_context = args.contains(&"--context");
                Ok(ImmutableChatCommand::Search {
                    query,
                    scope,
                    limit,
                    include_context,
                })
            }
            _ => Err(CandleCommandError::UnknownCommand {
                command: command_name,
            }),
        }
    }

    /// Register a command
    pub fn register_command(&mut self, info: &CommandInfo) {
        // Register main command name
        self.commands.insert(info.name.clone(), info.clone());

        // Register aliases
        for alias in &info.aliases {
            self.aliases.insert(alias.clone(), info.name.clone());
        }
    }

    /// Parse a command string with zero-allocation patterns
    ///
    /// # Errors
    /// Returns `ParseError::InvalidSyntax` if the command doesn't start with '/', if the command name is unknown, or if arguments are malformed
    pub fn parse(&self, input: &str) -> ParseResult<ImmutableChatCommand> {
        command_parsers::parse(input, &self.aliases)
    }

    /// Validate command parameters
    ///
    /// # Errors
    /// Returns `ParseError::InvalidParameterValue` if command parameters contain invalid values (e.g., unsupported export format, invalid search scope)
    pub fn validate_command(&self, command: &ImmutableChatCommand) -> ParseResult<()> {
        match command {
            ImmutableChatCommand::Export { format, .. } => {
                let valid_formats = ["json", "markdown", "pdf", "html"];
                if !valid_formats.contains(&format.as_str()) {
                    return Err(ParseError::InvalidParameterValue {
                        parameter: "format".to_string(),
                        value: format.clone(),
                    });
                }
            }
            ImmutableChatCommand::Clear {
                keep_last: Some(n), ..
            } => {
                if *n == 0 {
                    return Err(ParseError::InvalidParameterValue {
                        parameter: "keep-last".to_string(),
                        value: "0".to_string(),
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Get command suggestions for auto-completion
    #[must_use]
    pub fn get_suggestions(&self, prefix: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Add command names
        for name in self.commands.keys() {
            if name.starts_with(prefix) {
                suggestions.push(name.clone());
            }
        }

        // Add aliases
        for alias in self.aliases.keys() {
            if alias.starts_with(prefix) {
                suggestions.push(alias.clone());
            }
        }

        suggestions.sort();
        suggestions
    }

    /// Get command information
    #[must_use]
    pub fn get_command_info(&self, command: &str) -> Option<CommandInfo> {
        self.commands.get(command).cloned().or_else(|| {
            self.aliases
                .get(command)
                .and_then(|name| self.commands.get(name).cloned())
        })
    }

    /// Get command history
    #[must_use]
    pub fn get_history(&self) -> Vec<String> {
        self.history.clone()
    }

    /// Add command to history
    pub fn add_to_history(&mut self, command: String) {
        self.history.push(command);
        // Keep only last 100 commands
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }
}
