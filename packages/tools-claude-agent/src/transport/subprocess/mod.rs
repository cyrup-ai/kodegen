//! Subprocess transport implementation using Claude Code CLI
//!
//! This module provides a transport implementation that spawns the Claude Code CLI
//! as a subprocess and communicates with it via stdin/stdout.

mod command;
mod config;
mod lifecycle;
mod reader;
mod transport;

// Re-export public types
pub use config::PromptInput;
pub use transport::SubprocessTransport;
