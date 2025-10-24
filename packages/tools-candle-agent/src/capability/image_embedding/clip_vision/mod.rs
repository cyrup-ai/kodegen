//! CLIP Vision image embedding provider
//!
//! This module provides CLIP Vision models for encoding images to embeddings.
//! Supports ViT-Base-Patch32 (224×224, 512-dim) and ViT-Large-Patch14-336 (336×336, 768-dim).
//!
//! Uses ClipModel.get_image_features() for encoding images to embeddings.
//!
//! ## Module Structure
//!
//! - `config`: Static ModelInfo constants and config builders
//! - `models`: Core model structures (ClipVisionModel, LoadedClipVisionModel)
//! - `preprocessing`: Shared image preprocessing helpers (KEY deduplication module)
//! - `encoding`: ClipVisionModel encoding methods
//! - `loaded_encoding`: LoadedClipVisionModel helpers with spawn_blocking
//! - `traits`: ImageEmbeddingCapable trait implementations

mod config;
mod encoding;
mod loaded_encoding;
mod models;
mod preprocessing;
mod traits_lazy;
mod traits_loaded;

// Public exports
pub use config::{CLIP_VISION_BASE_INFO, CLIP_VISION_LARGE_INFO};
pub use models::{ClipVisionModel, LoadedClipVisionModel};

// Trait implementations are automatically available when the types are in scope
// No explicit re-export needed for trait impls
