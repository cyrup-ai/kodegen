//! High-level memory management functionality
//!
//! This module was decomposed from a 1,331-line monolithic file
//! into 9 focused modules for better maintainability.

mod conversions;
mod lifecycle;
mod operations;
mod relationships;
mod search;
mod temporal;
mod trait_impl;
mod types;
mod workers;

// Re-export public types
pub use types::*;

// Re-export the main coordinator struct
pub use lifecycle::MemoryCoordinator;
