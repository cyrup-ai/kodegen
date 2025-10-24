//! Macro system for chat automation with lock-free data structures
//!
//! This module provides a comprehensive macro system for recording, storing,
//! and playing back chat interactions using zero-allocation patterns and
//! lock-free data structures for blazing-fast performance.
//!
//! ## Architecture
//!
//! Decomposed from a 2,033-line monolithic file into focused modules:
//! - `types`: Core enums and type definitions
//! - `parser`: Conditional expression parsing
//! - `context`: Execution context and metadata
//! - `system`: `MacroSystem` implementation (recording/playback)
//! - `processor`: `MacroProcessor` implementation (advanced features)
//! - `errors`: Result and error types

pub mod context;
pub mod errors;
pub mod parser;
pub mod processor;
pub mod system;
pub mod types;

// Re-export all public items to preserve API
pub use context::*;
pub use errors::*;
pub use processor::*;
pub use system::*;
pub use types::*;
