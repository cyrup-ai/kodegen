//! Stella embedding provider for local inference using Candle ML framework
//!
//! This provider uses dunzhang/stella_en_400M_v5 or dunzhang/stella_en_1.5B_v5 models
//! for generating MRL-trained dimensional embeddings with ProgressHub download and Candle inference.
//!
//! Supports only trained MRL projection dimensions: 256, 768, 1024, 2048, 4096, 6144, 8192.
//! Architecture follows the real Candle EmbeddingModel pattern with native lm_head projections.

mod base;
mod config;
pub mod instruction;
mod loaded;
mod utils;

// Public API - maintain exact same exports as original stella.rs
pub use base::StellaEmbeddingModel;
pub use loaded::LoadedStellaModel;
