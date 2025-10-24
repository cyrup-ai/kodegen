//! SurrealDB memory manager implementation.
//!
//! This module was decomposed from a 2,062-line monolithic file into focused submodules
//! for better maintainability and separation of concerns.

pub mod futures;
pub mod manager;
pub mod operations;
pub mod queries;
pub mod trait_def;
pub mod types;

// Re-export all public items to maintain API compatibility
pub use futures::*;
pub use manager::*;
pub use trait_def::*;
pub use types::*;

// Result type used throughout the module
pub type Result<T> = std::result::Result<T, crate::memory::utils::error::Error>;
