//! Lightweight MCP tool schema definitions
//! 
//! This package contains ONLY Args and PromptArgs type definitions
//! with zero heavy dependencies. It serves as the single source of
//! truth for all tool schemas in the kodegen ecosystem.

pub mod filesystem;
pub mod terminal;
pub mod git;
pub mod github;
pub mod browser;
pub mod citescrape;
pub mod database;
pub mod reasoning;
pub mod claude_agent;
pub mod prompt;
pub mod introspection;
pub mod process;
pub mod config;

// Re-export all types at crate root for convenience
pub use filesystem::*;
pub use terminal::*;
pub use git::*;
pub use github::*;
pub use browser::*;
pub use citescrape::*;
pub use database::*;
pub use reasoning::*;
pub use claude_agent::*;
pub use prompt::*;
pub use introspection::*;
pub use process::*;
pub use config::*;
