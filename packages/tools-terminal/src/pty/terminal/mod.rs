// Core types and structures
mod types;
pub use types::{BellStyle, ColorMode, KeyCode, TermSize, Terminal, TerminalConfig};

// Builder pattern
mod builder;
pub use builder::TerminalBuilder;

// Factory methods
mod factory;

// PTY initialization
mod initialization;

// Input/output operations
mod io;

// Command execution
mod execution;

// Process management
mod process;

// Shell detection utilities
mod shell;
