//! Input handler for processing user messages and commands
//!
//! This module processes user input, handles commands (like /save, /search, /help),
//! and delegates to the existing chat infrastructure for message processing.

use super::completion::CommandCompleter;
use super::config::CliConfig;
use crate::domain::chat::CandleChatLoop;
use std::fs;
use std::path::Path;

/// Result of handling user input
#[derive(Debug, Clone)]
pub enum InputHandlerResult {
    /// Continue with chat message
    Chat(String),

    /// Execute command
    Command(CommandResult),

    /// Exit the application
    Exit,

    /// No action needed
    None,
}

/// Result of command execution
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Display help message
    Help(String),

    /// Save conversation to file
    SavedToFile(String),

    /// Search results
    SearchResults(Vec<String>),

    /// Configuration changed
    ConfigChanged(String),

    /// History cleared
    HistoryCleared,

    /// Error message
    Error(String),
}

/// Input handler for CLI
#[derive(Clone)]
pub struct InputHandler {
    config: CliConfig,
}

impl InputHandler {
    /// Create new input handler
    pub fn new(config: CliConfig) -> Self {
        Self { config }
    }

    /// Process user input and return action to take
    pub fn handle(&mut self, input: &str) -> InputHandlerResult {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return InputHandlerResult::None;
        }

        // Check if it's a command
        if CommandCompleter::is_command(trimmed) {
            return self.handle_command(trimmed);
        }

        // Regular chat message - add to history
        self.config.add_to_history(trimmed.to_string());

        InputHandlerResult::Chat(trimmed.to_string())
    }

    /// Handle command input
    fn handle_command(&mut self, input: &str) -> InputHandlerResult {
        let Some((command, args)) = CommandCompleter::parse_command(input) else {
            return InputHandlerResult::Command(CommandResult::Error(
                "Invalid command format".to_string(),
            ));
        };

        match command.as_str() {
            "/help" => self.handle_help(),
            "/exit" | "/quit" => InputHandlerResult::Exit,
            "/save" => self.handle_save(&args),
            "/search" => self.handle_search(&args),
            "/clear" => self.handle_clear(),
            "/history" => self.handle_history(),
            "/model" => self.handle_model_command(&args),
            "/system" => self.handle_system_command(&args),
            "/temperature" => self.handle_temperature(&args),
            "/tokens" => self.handle_tokens(&args),
            "/export" => self.handle_export(&args),
            "/import" => self.handle_import(&args),
            _ => InputHandlerResult::Command(CommandResult::Error(format!(
                "Unknown command: {}",
                command
            ))),
        }
    }

    /// Handle /help command
    fn handle_help(&self) -> InputHandlerResult {
        let help_text = r#"
Available Commands:
  /help           - Show this help message
  /exit, /quit    - Exit the application
  /save <file>    - Save conversation to file
  /search <query> - Search conversation history
  /clear          - Clear conversation history
  /history        - Show conversation history
  /model <name>   - Change current model
  /system <prompt>- Set system prompt
  /temperature <n>- Set temperature (0.0-2.0)
  /tokens <n>     - Set max tokens
  /export <file>  - Export configuration
  /import <file>  - Import configuration

Chat Commands:
  Type any message to chat with the AI
  Use arrow keys for history navigation
  Press Ctrl+C to cancel current message
"#;
        InputHandlerResult::Command(CommandResult::Help(help_text.to_string()))
    }

    /// Handle /save command
    fn handle_save(&self, args: &[String]) -> InputHandlerResult {
        if args.is_empty() {
            return InputHandlerResult::Command(CommandResult::Error(
                "Usage: /save <filename>".to_string(),
            ));
        }

        let filename = &args[0];
        let history = self.config.get_history();

        let content = history.join("\n\n---\n\n");

        match fs::write(filename, content) {
            Ok(_) => InputHandlerResult::Command(CommandResult::SavedToFile(filename.clone())),
            Err(e) => {
                InputHandlerResult::Command(CommandResult::Error(format!("Failed to save: {}", e)))
            }
        }
    }

    /// Handle /search command
    fn handle_search(&self, args: &[String]) -> InputHandlerResult {
        if args.is_empty() {
            return InputHandlerResult::Command(CommandResult::Error(
                "Usage: /search <query>".to_string(),
            ));
        }

        let query = args.join(" ");
        let history = self.config.get_history();

        let results: Vec<String> = history
            .iter()
            .filter(|msg| msg.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();

        InputHandlerResult::Command(CommandResult::SearchResults(results))
    }

    /// Handle /clear command
    fn handle_clear(&mut self) -> InputHandlerResult {
        self.config.clear_history();
        InputHandlerResult::Command(CommandResult::HistoryCleared)
    }

    /// Handle /history command
    fn handle_history(&self) -> InputHandlerResult {
        let history = self.config.get_history();
        InputHandlerResult::Command(CommandResult::SearchResults(history.to_vec()))
    }

    /// Handle /model command
    fn handle_model_command(&mut self, args: &[String]) -> InputHandlerResult {
        if args.is_empty() {
            return InputHandlerResult::Command(CommandResult::Error(
                "Usage: /model <registry_key>".to_string(),
            ));
        }

        let model = args[0].clone();
        self.config.set_last_model(model.clone());

        InputHandlerResult::Command(CommandResult::ConfigChanged(format!(
            "Model changed to: {}",
            model
        )))
    }

    /// Handle /system command
    fn handle_system_command(&mut self, args: &[String]) -> InputHandlerResult {
        if args.is_empty() {
            return InputHandlerResult::Command(CommandResult::Error(
                "Usage: /system <prompt>".to_string(),
            ));
        }

        let prompt = args.join(" ");
        self.config.default_system_prompt = Some(prompt.clone());

        InputHandlerResult::Command(CommandResult::ConfigChanged(
            "System prompt updated".to_string(),
        ))
    }

    /// Handle /temperature command
    fn handle_temperature(&mut self, args: &[String]) -> InputHandlerResult {
        if args.is_empty() {
            return InputHandlerResult::Command(CommandResult::Error(
                "Usage: /temperature <0.0-2.0>".to_string(),
            ));
        }

        match args[0].parse::<f64>() {
            Ok(temp) if (0.0..=2.0).contains(&temp) => {
                self.config.default_temperature = temp;
                InputHandlerResult::Command(CommandResult::ConfigChanged(format!(
                    "Temperature set to: {}",
                    temp
                )))
            }
            Ok(temp) => InputHandlerResult::Command(CommandResult::Error(format!(
                "Temperature must be between 0.0 and 2.0, got {}",
                temp
            ))),
            Err(e) => InputHandlerResult::Command(CommandResult::Error(format!(
                "Invalid temperature: {}",
                e
            ))),
        }
    }

    /// Handle /tokens command
    fn handle_tokens(&mut self, args: &[String]) -> InputHandlerResult {
        if args.is_empty() {
            return InputHandlerResult::Command(CommandResult::Error(
                "Usage: /tokens <number>".to_string(),
            ));
        }

        match args[0].parse::<u64>() {
            Ok(tokens) if tokens > 0 => {
                self.config.default_max_tokens = tokens;
                InputHandlerResult::Command(CommandResult::ConfigChanged(format!(
                    "Max tokens set to: {}",
                    tokens
                )))
            }
            Ok(_) => InputHandlerResult::Command(CommandResult::Error(
                "Max tokens must be greater than 0".to_string(),
            )),
            Err(e) => {
                InputHandlerResult::Command(CommandResult::Error(format!("Invalid number: {}", e)))
            }
        }
    }

    /// Handle /export command
    fn handle_export(&self, args: &[String]) -> InputHandlerResult {
        if args.is_empty() {
            return InputHandlerResult::Command(CommandResult::Error(
                "Usage: /export <filename>".to_string(),
            ));
        }

        let path = Path::new(&args[0]);
        match self.config.save(Some(path)) {
            Ok(_) => InputHandlerResult::Command(CommandResult::SavedToFile(args[0].clone())),
            Err(e) => {
                InputHandlerResult::Command(CommandResult::Error(format!("Export failed: {}", e)))
            }
        }
    }

    /// Handle /import command
    fn handle_import(&mut self, args: &[String]) -> InputHandlerResult {
        if args.is_empty() {
            return InputHandlerResult::Command(CommandResult::Error(
                "Usage: /import <filename>".to_string(),
            ));
        }

        let path = Path::new(&args[0]);
        match CliConfig::load(Some(path)) {
            Ok(config) => {
                self.config = config;
                InputHandlerResult::Command(CommandResult::ConfigChanged(
                    "Configuration imported".to_string(),
                ))
            }
            Err(e) => {
                InputHandlerResult::Command(CommandResult::Error(format!("Import failed: {}", e)))
            }
        }
    }

    /// Get current config
    pub fn config(&self) -> &CliConfig {
        &self.config
    }

    /// Get mutable config
    pub fn config_mut(&mut self) -> &mut CliConfig {
        &mut self.config
    }

    /// Convert input to CandleChatLoop for the agent
    pub fn to_chat_loop(&self, input: InputHandlerResult) -> Option<CandleChatLoop> {
        match input {
            InputHandlerResult::Chat(message) => Some(CandleChatLoop::UserPrompt(message)),
            InputHandlerResult::Exit => Some(CandleChatLoop::Break),
            _ => None,
        }
    }
}
