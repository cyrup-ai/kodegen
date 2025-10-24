//! Vision (Multimodal) Capability
//!
//! Providers that implement vision/multimodal capabilities (text generation from images).

pub mod llava;

// Re-exports for convenience
pub(crate) use llava::LLaVAModel;
