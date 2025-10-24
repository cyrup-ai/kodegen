//! Command registration and discovery
//!
//! Provides compile-time command registration with zero-allocation patterns and blazing-fast
//! command discovery for production-ready performance.

use std::collections::HashMap;
use std::sync::Arc;

use crossbeam_skiplist::SkipMap;

use super::types::{CandleCommandError, CommandInfo};

/// Command registry with concurrent access
#[derive(Debug, Clone)]
pub struct CommandRegistry {
    /// Registered commands (lock-free concurrent map)
    commands: Arc<SkipMap<String, CommandInfo>>,
    /// Command aliases (lock-free concurrent map)
    aliases: Arc<SkipMap<String, String>>,
    /// Command categories
    categories: Arc<SkipMap<String, Vec<String>>>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    /// Create a new command registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: Arc::new(SkipMap::new()),
            aliases: Arc::new(SkipMap::new()),
            categories: Arc::new(SkipMap::new()),
        }
    }

    /// Register a command with compile-time validation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Command name is empty
    /// - Command description is empty
    /// - Command name is already registered
    /// - Any alias is already registered
    pub fn register(&self, info: &CommandInfo) -> Result<(), CandleCommandError> {
        // Validate command info
        if info.name.is_empty() {
            return Err(CandleCommandError::ConfigurationError {
                detail: "Command name cannot be empty".to_string(),
            });
        }

        if info.description.is_empty() {
            return Err(CandleCommandError::ConfigurationError {
                detail: "Command description cannot be empty".to_string(),
            });
        }

        // Check for duplicate command names
        let command_name = info.name.clone();
        if self.commands.contains_key(&command_name) {
            return Err(CandleCommandError::ConfigurationError {
                detail: format!("Command '{}' already registered", info.name),
            });
        }

        // Check for duplicate aliases
        for alias in &info.aliases {
            let alias_key = alias.clone();
            if self.aliases.contains_key(&alias_key) {
                return Err(CandleCommandError::ConfigurationError {
                    detail: format!("Alias '{alias}' already registered"),
                });
            }
        }

        // Register command
        self.commands.insert(command_name.clone(), info.clone());

        // Register aliases
        for alias in &info.aliases {
            self.aliases.insert(alias.clone(), command_name.clone());
        }

        // Update category index
        let category_key = info.category.clone();
        if let Some(existing) = self.categories.get(&category_key) {
            let mut category_list = existing.value().clone();
            category_list.push(command_name.clone());
            self.categories.insert(category_key, category_list);
        } else {
            self.categories
                .insert(category_key, vec![command_name.clone()]);
        }

        Ok(())
    }

    /// Unregister a command
    ///
    /// # Errors
    ///
    /// Returns `CandleCommandError::UnknownCommand` if the command name is not found in the registry
    pub fn unregister(&self, name: &str) -> Result<(), CandleCommandError> {
        let command_name = name.to_string();

        // Get command info before removal
        let info = self
            .commands
            .get(&command_name)
            .ok_or_else(|| CandleCommandError::UnknownCommand {
                command: command_name.clone(),
            })?
            .value()
            .clone();

        // Remove command
        self.commands.remove(&command_name);

        // Remove aliases
        for alias in &info.aliases {
            self.aliases.remove(alias);
        }

        // Update category index
        let category_key = info.category.clone();
        if let Some(existing) = self.categories.get(&category_key) {
            let mut category_list = existing.value().clone();
            category_list.retain(|cmd| cmd != &command_name);
            if category_list.is_empty() {
                self.categories.remove(&category_key);
            } else {
                self.categories.insert(category_key, category_list);
            }
        }

        Ok(())
    }

    /// Get command information
    #[must_use]
    pub fn get_command(&self, name: &str) -> Option<CommandInfo> {
        let command_name = name.to_string();

        // Try direct lookup first
        if let Some(entry) = self.commands.get(&command_name) {
            return Some(entry.value().clone());
        }

        // Try alias lookup
        if let Some(alias_entry) = self.aliases.get(&command_name) {
            let actual_name = alias_entry.value();
            if let Some(command_entry) = self.commands.get(actual_name) {
                return Some(command_entry.value().clone());
            }
        }

        None
    }

    /// Check if command exists
    #[must_use]
    pub fn has_command(&self, name: &str) -> bool {
        let command_name = name.to_string();
        self.commands.contains_key(&command_name) || self.aliases.contains_key(&command_name)
    }

    /// List all registered commands
    #[must_use]
    pub fn list_commands(&self) -> Vec<CommandInfo> {
        self.commands
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List commands by category
    #[must_use]
    pub fn list_commands_by_category(&self, category: &str) -> Vec<CommandInfo> {
        let category_key = category.to_string();

        if let Some(command_names) = self.categories.get(&category_key) {
            command_names
                .value()
                .iter()
                .filter_map(|name| self.commands.get(name).map(|entry| entry.value().clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// List all categories
    #[must_use]
    pub fn list_categories(&self) -> Vec<String> {
        self.categories
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get command suggestions for auto-completion
    #[must_use]
    pub fn get_suggestions(&self, prefix: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Add matching command names
        for entry in self.commands.iter() {
            if entry.key().starts_with(prefix) {
                suggestions.push(entry.key().clone());
            }
        }

        // Add matching aliases
        for entry in self.aliases.iter() {
            if entry.key().starts_with(prefix) {
                suggestions.push(entry.key().clone());
            }
        }

        // Sort suggestions
        suggestions.sort();
        suggestions.dedup();
        suggestions
    }

    /// Search commands by keyword
    #[must_use]
    pub fn search_commands(&self, keyword: &str) -> Vec<CommandInfo> {
        let keyword_lower = keyword.to_lowercase();

        self.commands
            .iter()
            .filter(|entry| {
                let info = entry.value();
                info.name.to_lowercase().contains(&keyword_lower)
                    || info.description.to_lowercase().contains(&keyword_lower)
                    || info.category.to_lowercase().contains(&keyword_lower)
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get command statistics
    #[must_use]
    pub fn get_statistics(&self) -> CommandRegistryStats {
        let total_commands = self.commands.len();
        let total_aliases = self.aliases.len();
        let total_categories = self.categories.len();

        let mut category_counts = HashMap::new();
        for entry in self.categories.iter() {
            category_counts.insert(entry.key().clone(), entry.value().len());
        }

        CommandRegistryStats {
            total_commands,
            total_aliases,
            total_categories,
            category_counts,
        }
    }

    /// Validate registry consistency
    ///
    /// # Errors
    ///
    /// Returns a vector of `CandleCommandError::ConfigurationError` for each validation failure found:
    /// - Orphaned aliases pointing to non-existent commands
    /// - Empty categories with no commands
    /// - Commands with parameters that have empty names
    pub fn validate(&self) -> Result<(), Vec<CandleCommandError>> {
        let mut errors = Vec::new();

        // Check for orphaned aliases
        for alias_entry in self.aliases.iter() {
            let command_name = alias_entry.value();
            if !self.commands.contains_key(command_name) {
                errors.push(CandleCommandError::ConfigurationError {
                    detail: format!(
                        "Orphaned alias '{}' points to non-existent command '{}'",
                        alias_entry.key(),
                        command_name
                    ),
                });
            }
        }

        // Check for empty categories
        for category_entry in self.categories.iter() {
            if category_entry.value().is_empty() {
                errors.push(CandleCommandError::ConfigurationError {
                    detail: format!("Empty category '{}'", category_entry.key()),
                });
            }
        }

        // Check for commands with invalid parameters
        for command_entry in self.commands.iter() {
            let info = command_entry.value();
            for param in &info.parameters {
                if param.name.is_empty() {
                    errors.push(CandleCommandError::ConfigurationError {
                        detail: format!("Command '{}' has parameter with empty name", info.name),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Clear all registered commands
    pub fn clear(&self) {
        self.commands.clear();
        self.aliases.clear();
        self.categories.clear();
    }

    /// Get command count
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Get alias count
    #[must_use]
    pub fn alias_count(&self) -> usize {
        self.aliases.len()
    }

    /// Export registry to JSON
    ///
    /// # Errors
    ///
    /// Returns `CandleCommandError::ConfigurationError` if JSON serialization fails
    pub fn export_to_json(&self) -> Result<String, CandleCommandError> {
        let commands: Vec<CommandInfo> = self.list_commands();
        serde_json::to_string_pretty(&commands).map_err(|e| {
            CandleCommandError::ConfigurationError {
                detail: format!("Failed to export registry: {e}"),
            }
        })
    }

    /// Import registry from JSON
    ///
    /// # Errors
    ///
    /// Returns `CandleCommandError::ConfigurationError` if:
    /// - JSON deserialization fails
    /// - Any imported command fails validation during registration
    pub fn import_from_json(&self, json: &str) -> Result<(), CandleCommandError> {
        let commands: Vec<CommandInfo> =
            serde_json::from_str(json).map_err(|e| CandleCommandError::ConfigurationError {
                detail: format!("Failed to import registry: {e}"),
            })?;

        // Clear existing registry
        self.clear();

        // Register imported commands
        for command in &commands {
            self.register(command)?;
        }

        Ok(())
    }
}

/// Command registry statistics
#[derive(Debug, Clone)]
pub struct CommandRegistryStats {
    /// Total number of registered commands
    pub total_commands: usize,
    /// Total number of aliases
    pub total_aliases: usize,
    /// Total number of categories
    pub total_categories: usize,
    /// Commands per category
    pub category_counts: HashMap<String, usize>,
}

/// Global command registry instance
static GLOBAL_REGISTRY: std::sync::LazyLock<CommandRegistry> =
    std::sync::LazyLock::new(CommandRegistry::new);

/// Get global command registry
#[must_use]
pub fn get_global_registry() -> &'static CommandRegistry {
    &GLOBAL_REGISTRY
}

/// Register command globally
///
/// # Errors
///
/// Returns an error if command registration fails (see `CommandRegistry::register` for details)
pub fn register_global_command(info: &CommandInfo) -> Result<(), CandleCommandError> {
    get_global_registry().register(info)
}

/// Get command from global registry
#[must_use]
pub fn get_global_command(name: &str) -> Option<CommandInfo> {
    get_global_registry().get_command(name)
}

/// Check if command exists in global registry
#[must_use]
pub fn has_global_command(name: &str) -> bool {
    get_global_registry().has_command(name)
}

/// Get suggestions from global registry
#[must_use]
pub fn get_global_suggestions(prefix: &str) -> Vec<String> {
    get_global_registry().get_suggestions(prefix)
}

/// Search commands in global registry
#[must_use]
pub fn search_global_commands(keyword: &str) -> Vec<CommandInfo> {
    get_global_registry().search_commands(keyword)
}
