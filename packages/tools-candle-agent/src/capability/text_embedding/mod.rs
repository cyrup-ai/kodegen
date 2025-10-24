//! Text Embedding Capability
//!
//! Providers that implement text embedding using EmbeddingModel trait.

pub mod safetensors_validation;

pub mod stella;

// Re-exports for convenience
pub(crate) use stella::StellaEmbeddingModel;
