// Public modules
pub mod terminal;

// Re-export tokio for async runtime
pub use tokio;

// Core terminal types
pub use terminal::{BellStyle, ColorMode, KeyCode, TermSize, Terminal, TerminalBuilder};

// External re-exports
pub use portable_pty::CommandBuilder;
