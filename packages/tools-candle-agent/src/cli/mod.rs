//! CLI module for interactive chat applications
//!
//! This module provides a complete CLI framework for building interactive AI chat applications
//! using the inquire library for rich terminal UX with autocompletion, fuzzy matching, and
//! history management.

pub mod args;
pub mod completion;
pub mod config;
pub mod handler;
pub mod prompt;
pub mod runner;

// Re-export main types for convenience
pub use args::CliArgs;
pub use completion::{CommandCompleter, ModelCompleter};
pub use config::CliConfig;
pub use handler::{CommandResult, InputHandler, InputHandlerResult};
pub use prompt::PromptBuilder;
pub use runner::CliRunner;
