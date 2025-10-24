//! Generation module - Modular text generation with SIMD acceleration
//!
//! This module provides a comprehensive text generation system built on:
//! - AsyncStream-based token streaming (no futures)
//! - SIMD-optimized sampling and processing
//! - Modular architecture with clear separation of concerns
//! - Comprehensive statistics and performance monitoring
//!
//! ## Module Organization
//!
//! - [`types`] - Core types, aliases and constants
//! - [`tokens`] - Token management and special token handling
//! - [`config`] - Sampling configuration and parameter management
//! - [`stats`] - Generation statistics and performance monitoring
//! - [`metrics`] - SIMD-specific performance metrics
//! - [`models`] - Model integration and wrapper functionality
//! - [`generator`] - Core text generation engine
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use candle_generation::{
//!     TextGenerator, SamplingConfig, SpecialTokens,
//!     CandleLlamaModel, ModelFactory
//! };
//!
//! // Create configuration
//! let config = SamplingConfig::new(0.8)
//!     .with_top_k(40)
//!     .with_top_p(0.9);
//!
//! // Create model and generator
//! let model = ModelFactory::create_llama(model_config, device)?;
//! let mut generator = TextGenerator::new(
//!     Box::new(model), tokenizer, device, config
//! );
//!
//! // Generate text with AsyncStream
//! let tokens = SpecialTokens::with_eos(2);
//! let stream = generator.generate("Hello world".to_string(), 100, tokens);
//!
//! // Process stream
//! stream.for_each(|token| {
//!     print!("{}", token);
//! });
//! ```

// Public module declarations
pub mod config;
pub mod generator;
pub mod metrics;
pub mod models;
pub mod stats;
pub mod token_output_stream;
pub mod tokens;
pub mod types;

// Re-export core types for ergonomic usage
pub use config::{
    SamplingConfig, balanced_config, creative_config, deterministic_config, focused_config,
};
pub use generator::TextGenerator;
pub use metrics::SimdMetrics;
pub use models::{
    CandleLlamaModel, CandleModel, CandleQuantizedLlamaModel, CandleQuantizedMixFormerModel,
    CandleQuantizedPhiModel,
};
pub use stats::GenerationStatistics;
pub use token_output_stream::TokenOutputStream;
pub use tokens::{SpecialTokens, TokenHistory, TokenProb};
pub use types::{CandleResult, LogitsBuffer, SAMPLING_CACHE_SIZE, SIMD_THRESHOLD};
