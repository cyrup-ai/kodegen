//! Image Embedding Capability
//!
//! Providers that implement image embedding using ImageEmbeddingCapable trait.

pub mod clip_vision;
pub mod clip_vision_embedding;

// Re-exports for convenience
pub(crate) use clip_vision::{ClipVisionModel, LoadedClipVisionModel};
pub(crate) use clip_vision_embedding::ClipVisionEmbeddingModel;
