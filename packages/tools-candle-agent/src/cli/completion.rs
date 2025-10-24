//! Autocompletion for models and commands using fuzzy matching
//!
//! This module provides intelligent autocompletion for model selection and chat commands
//! using the fuzzy-matcher library for approximate string matching.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

/// Available chat commands
pub const CHAT_COMMANDS: &[&str] = &[
    "/save",
    "/search",
    "/help",
    "/clear",
    "/exit",
    "/quit",
    "/model",
    "/system",
    "/temperature",
    "/tokens",
    "/history",
    "/export",
    "/import",
];

/// Model completer with fuzzy matching
#[derive(Clone)]
pub struct ModelCompleter {
    matcher: std::sync::Arc<SkimMatcherV2>,
    models: Vec<String>,
    aliases: Vec<String>,
}

impl ModelCompleter {
    /// Create new model completer
    pub fn new() -> Self {
        let models = vec![
            "unsloth/Phi-4-reasoning-GGUF".to_string(),
            "unsloth/Kimi-K2-Instruct-GGUF".to_string(),
            "Qwen/Qwen2.5-Coder-32B-Instruct-GGUF".to_string(),
        ];

        let aliases = vec![
            "phi4".to_string(),
            "phi-4".to_string(),
            "phi4-reasoning".to_string(),
            "kimi".to_string(),
            "kimi-k2".to_string(),
            "qwen".to_string(),
            "qwen-coder".to_string(),
            "qwen3-coder".to_string(),
        ];

        Self {
            matcher: std::sync::Arc::new(SkimMatcherV2::default()),
            models,
            aliases,
        }
    }

    /// Get all available options (models + aliases)
    pub fn all_options(&self) -> Vec<String> {
        let mut options = self.models.clone();
        options.extend(self.aliases.clone());
        options
    }

    /// Autocomplete model name with fuzzy matching
    pub fn complete(&self, input: &str) -> Vec<String> {
        if input.is_empty() {
            return self.all_options();
        }

        let mut results: Vec<(i64, String)> = self
            .all_options()
            .into_iter()
            .filter_map(|option| {
                self.matcher
                    .fuzzy_match(&option, input)
                    .map(|score| (score, option))
            })
            .collect();

        // Sort by score (descending)
        results.sort_by(|a, b| b.0.cmp(&a.0));

        results.into_iter().map(|(_, option)| option).collect()
    }

    /// Get best match for input
    pub fn best_match(&self, input: &str) -> Option<String> {
        self.complete(input).into_iter().next()
    }
}

impl Default for ModelCompleter {
    fn default() -> Self {
        Self::new()
    }
}

/// Command completer for chat commands
#[derive(Clone)]
pub struct CommandCompleter {
    matcher: std::sync::Arc<SkimMatcherV2>,
}

impl CommandCompleter {
    /// Create new command completer
    pub fn new() -> Self {
        Self {
            matcher: std::sync::Arc::new(SkimMatcherV2::default()),
        }
    }

    /// Autocomplete command with fuzzy matching
    pub fn complete(&self, input: &str) -> Vec<String> {
        if !input.starts_with('/') {
            return vec![];
        }

        let mut results: Vec<(i64, String)> = CHAT_COMMANDS
            .iter()
            .filter_map(|cmd| {
                self.matcher
                    .fuzzy_match(cmd, input)
                    .map(|score| (score, cmd.to_string()))
            })
            .collect();

        // Sort by score (descending)
        results.sort_by(|a, b| b.0.cmp(&a.0));

        results.into_iter().map(|(_, cmd)| cmd).collect()
    }

    /// Check if input is a command
    pub fn is_command(input: &str) -> bool {
        input.starts_with('/')
    }

    /// Parse command and arguments
    pub fn parse_command(input: &str) -> Option<(String, Vec<String>)> {
        if !Self::is_command(input) {
            return None;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let command = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        Some((command, args))
    }

    /// Get all available commands
    pub fn all_commands(&self) -> Vec<String> {
        CHAT_COMMANDS.iter().map(|s| s.to_string()).collect()
    }
}

impl Default for CommandCompleter {
    fn default() -> Self {
        Self::new()
    }
}
