//! Fluent AI Candle Library
#![recursion_limit = "256"]
#![feature(impl_trait_in_fn_trait_return)]
#![feature(impl_trait_in_assoc_type)]
#![feature(negative_impls)]
#![feature(auto_traits)]
#![feature(fn_traits)]

//! This crate provides Candle ML framework integration for AI services.
//! All Candle-prefixed domain types, builders, and providers are defined here
//! to ensure complete independence from the main cyrup packages.

// Initialize performance optimizations on library load
use std::sync::Once;
static INIT: Once = Once::new();

/// Initialize library-wide performance optimizations
pub fn init_candle() {
    INIT.call_once(|| {
        // Initialize timestamp caching for high-performance operations
        domain::memory::cache::initialize_timestamp_cache();

        // Initialize memory node pool for zero-allocation memory operations
        // Using 1000 nodes with 768-dimensional embeddings (typical for BERT-base)
        domain::memory::pool::initialize_memory_node_pool(1000, 768);
    });
}

pub mod macros;

// Candle-specific modules (minimal set for core functionality)
/// Async stream utilities using tokio streams
pub mod async_stream;
/// Candle builders for zero-allocation construction patterns
pub mod builders;
/// Candle macros for ARCHITECTURE.md syntax support
/// Candle capabilities organized by what models can do
pub mod capability;
/// CLI module for interactive chat applications
pub mod cli;
/// Chat functionality is now available through domain::chat
/// Core components (engine, generation, etc.)
pub mod core;
/// Candle domain types (replaces cyrup_domain dependency)
pub mod domain;
/// Extension integration for Raycast and Alfred (macOS)
pub mod extensions;
/// Image processing utilities
pub mod image;
/// Memory system with cognitive features and vector storage
pub mod memory;
/// MCP tools for memory operations
pub mod tools;
/// Prompt processing utilities
pub mod prompt;
/// Shared Tokio runtime for avoiding multiple runtime creation
pub mod runtime;
/// Utility modules for common operations
pub mod util;
/// Real workflow execution system with streams-only architecture
pub mod workflow;

// Essential Candle re-exports for public API (minimal set)
// Domain types will be added as they become available

// Prelude - All types needed for ARCHITECTURE.md syntax
pub mod prelude {
    pub use crate::builders::{CandleAgentBuilder, CandleAgentRoleBuilder, CandleFluentAi};
    // Vision builder for image description
    pub use crate::builders::CandleVisionBuilder;
    // Embedding builder for text embeddings
    pub use crate::builders::EmbeddingBuilder;
    pub use crate::domain::Embedding;
    // Re-export generation types from modular structure
    pub use crate::core::generation::{
        CandleLlamaModel, CandleModel, GenerationStatistics, SamplingConfig, SimdMetrics,
        SpecialTokens, TextGenerator, TokenHistory,
    };
    pub use crate::core::{
        Engine, EngineConfig, EngineError, EngineResult, ModelArchitecture, ModelConfig,
        ModelConfigError,
    };
    pub use crate::domain::chat::CandleChatLoop;
    pub use crate::domain::chat::message::CandleMessageChunk;
    pub use crate::domain::{
        agent::CandleAgent,
        chat::message::types::CandleMessageRole,
        context::{
            FinishReason,
            chunks::CandleStringChunk,
            provider::{CandleContext, CandleDirectory, CandleFile, CandleFiles, CandleGithub},
        },
        image_generation::{
            ImageGenerationChunk, ImageGenerationConfig, ImageGenerationModel, tensor_to_image,
        },
        tool::{CandleToolRouter, CyloBackendConfig, RouterError},
    };

    // Re-export workspace MCP types for convenience
    pub use kodegen_mcp_client::KodegenClient;
    pub use kodegen_mcp_tool::Tool;
    pub use rmcp::model::Tool as ToolInfo;

    // Real workflow execution types - streams-only architecture
    pub use crate::workflow::{CandleExecutableWorkflow, CandleWorkflowStep, candle_workflow};

    // Pool infrastructure for transparent worker management
    pub use crate::capability::registry::pool::{Pool, PoolError, init_maintenance};

    pub struct CandleLibrary;

    impl CandleLibrary {
        pub fn named(_name: &str) -> Self {
            Self
        }
    }

    // Re-export tool implementation that provides static methods

    // Helper function for ARCHITECTURE.md example
    pub fn process_turn() -> CandleChatLoop {
        CandleChatLoop::Reprompt("continue".to_string())
    }
}

// Re-export everything from prelude at root level for convenience
// Re-export tokio_stream for convenience
pub use tokio_stream::{Stream, StreamExt};

// Re-export our stream utilities
pub use crate::async_stream::{empty, from_iter, once, spawn_stream};
// SIMD operations from cyrup-simd for high-performance ML workloads
pub use kodegen_simd;
pub use prelude::*;

// Pool infrastructure (part of registry)
pub use capability::registry::pool::{Pool, PoolError, init_maintenance};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_md_syntax_works() {
        // Test that ARCHITECTURE.md builder pattern still works after all fixes
        let _agent = CandleFluentAi::agent_role("test-agent")
            .temperature(0.0) // Greedy sampling example - deterministic output
            .max_tokens(1000)
            .system_prompt("You are a helpful assistant")
            .into_agent();

        // If this compiles, the ARCHITECTURE.md syntax is working! ✅
    }
}
