//! Fluent AI Candle Domain Library
//!
//! This crate provides Candle-prefixed domain types and traits for AI services.
//! All domain logic, message types, and business objects are defined here with Candle prefixes
//! to ensure complete independence from the main cyrup domain package.

#![warn(rustdoc::missing_crate_level_docs)]
#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

// Core modules and submodules
pub mod additional_types;
pub mod agent;
pub mod chat;
pub mod collections;
pub mod completion;
pub mod concurrency;
pub mod context;
pub mod core;
pub mod embedding;
pub mod embedding_result;
pub mod error;
/// Image processing and vision model support
pub mod image;
/// Image generation domain types for text-to-image diffusion models
pub mod image_generation;
pub mod init;
pub mod memory;
pub mod model;
/// Prompt construction and templating
pub mod prompt;
pub mod tool;
pub mod util;
pub mod voice;
// Use ZeroOneOrMany from cyrup_sugars directly
// Re-export from cyrup_sugars for convenience with Candle prefixes
pub use cyrup_sugars::{ByteSize, OneOrMany};
// Re-export HashMap from hashbrown for domain consistency
pub use hashbrown::HashMap;
// Re-export streaming types for backward compatibility
pub use tokio_stream::Stream;

// Re-export embedding result type
pub use embedding_result::Embedding;

// Re-export only from minimal working modules
// Most re-exports temporarily disabled until import issues resolved
