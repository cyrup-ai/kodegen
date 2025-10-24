//! Builder patterns for candle components
//!
//! This module contains candle-specific builder patterns following cyrup
//! architecture but with candle prefixes. NO trait objects allowed - only
//! impl Trait patterns for zero allocation.

pub mod agent_role;
pub mod completion;
pub mod document;
pub mod embedding;
pub mod extractor;
pub mod image;
pub mod vision;

// Re-export main builder types for public API
pub use agent_role::{CandleAgentBuilder, CandleAgentRoleBuilder, CandleFluentAi};
pub use embedding::EmbeddingBuilder;
pub use extractor::{ExtractorBuilder, extractor};
pub use image::ResizeFilter;
pub use vision::CandleVisionBuilder;
