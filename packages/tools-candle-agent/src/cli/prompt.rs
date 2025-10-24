//! Interactive prompt builder using inquire library
//!
//! This module provides a wrapper around inquire for creating rich terminal prompts
//! with autocompletion, fuzzy matching, and history management.

use super::completion::{CommandCompleter, ModelCompleter};
use inquire::{Select, Text};

/// Prompt builder for interactive CLI
#[derive(Clone)]
pub struct PromptBuilder {
    model_completer: ModelCompleter,
    command_completer: CommandCompleter,
}

impl PromptBuilder {
    /// Create new prompt builder
    pub fn new() -> Self {
        Self {
            model_completer: ModelCompleter::new(),
            command_completer: CommandCompleter::new(),
        }
    }

    /// Prompt for model selection with fuzzy search
    pub fn select_model(&self, default: Option<&str>) -> Result<String, String> {
        let options = self.model_completer.all_options();

        if options.is_empty() {
            return Err("No models available".to_string());
        }

        let mut select = Select::new("Select a model:", options);

        // Set default if provided and exists in options
        if let Some(def) = default
            && let Some(best_match) = self.model_completer.best_match(def)
        {
            select = select.with_starting_cursor(0);
            // Find index of best match
            if let Some(pos) = self
                .model_completer
                .all_options()
                .iter()
                .position(|m| m == &best_match)
            {
                select = select.with_starting_cursor(pos);
            }
        }

        select
            .prompt()
            .map_err(|e| format!("Model selection failed: {}", e))
    }

    /// Prompt for user input with command autocompletion
    pub fn get_user_input(&self, prompt: &str) -> Result<String, String> {
        Text::new(prompt)
            .prompt()
            .map_err(|e| format!("Input prompt failed: {}", e))
    }

    /// Prompt for system prompt configuration
    pub fn get_system_prompt(&self, default: Option<&str>) -> Result<String, String> {
        let mut text = Text::new("Enter system prompt:");

        if let Some(def) = default {
            text = text.with_default(def);
        }

        text.prompt()
            .map_err(|e| format!("System prompt failed: {}", e))
    }

    /// Prompt for temperature value
    pub fn get_temperature(&self, default: f64) -> Result<f64, String> {
        let input = Text::new("Temperature (0.0-2.0):")
            .with_default(&default.to_string())
            .prompt()
            .map_err(|e| format!("Temperature prompt failed: {}", e))?;

        input
            .parse::<f64>()
            .map_err(|e| format!("Invalid temperature: {}", e))
    }

    /// Prompt for max tokens
    pub fn get_max_tokens(&self, default: u64) -> Result<u64, String> {
        let input = Text::new("Max tokens:")
            .with_default(&default.to_string())
            .prompt()
            .map_err(|e| format!("Max tokens prompt failed: {}", e))?;

        input
            .parse::<u64>()
            .map_err(|e| format!("Invalid max tokens: {}", e))
    }

    /// Prompt for yes/no confirmation
    pub fn confirm(&self, message: &str) -> Result<bool, String> {
        inquire::Confirm::new(message)
            .with_default(true)
            .prompt()
            .map_err(|e| format!("Confirmation failed: {}", e))
    }

    /// Check if input is a command and get suggestions
    pub fn suggest_commands(&self, input: &str) -> Vec<String> {
        if CommandCompleter::is_command(input) {
            self.command_completer.complete(input)
        } else {
            vec![]
        }
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}
