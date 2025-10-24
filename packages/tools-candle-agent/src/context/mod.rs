//! Unified Context Module
//!
//! This module consolidates all context-related functionality including:
//! - Document types and loading (from document.rs)
//! - Chunk types for streaming operations (from chunk.rs)
//! - Production-ready context management with memory integration (from context.rs)
//! - Core file loading interface and implementation (from loader.rs)
//! - Structured data extraction from unstructured text (from extraction/)
//!
//! The module provides a clean, unified interface for all context operations
//! while maintaining high performance and memory integration capabilities.

pub mod chunk;
pub mod document;
pub mod extraction;
pub mod loader;

// Re-export all types for easy access
pub use chunk::*;
pub use document::*;
pub use extraction::*;
pub use loader::*;
