//! Quantum-inspired cognitive types for memory systems
//!
//! This module was decomposed from a 1,970-line monolithic `types.rs` file
//! into 8 focused modules for better maintainability.
//!
//! ## Modules
//! - `activation`: SIMD-aligned activation patterns
//! - `attention`: Atomic attention weight management
//! - `memory_items`: Memory item definitions
//! - `temporal`: Temporal and causal context
//! - `quantum`: Quantum signatures and entanglement
//! - `atomics`: Atomic wrapper types
//! - `state`: Core cognitive state
//! - `processor`: High-level processing system

pub mod activation;
pub mod atomics;
pub mod attention;
pub mod memory_items;
pub mod processor;
pub mod quantum;
pub mod state;
pub mod temporal;

// Re-export all public items to maintain API compatibility
pub use activation::*;
pub use atomics::*;
pub use attention::*;
pub use memory_items::*;
pub use processor::*;
pub use quantum::*;
pub use state::*;
pub use temporal::*;
