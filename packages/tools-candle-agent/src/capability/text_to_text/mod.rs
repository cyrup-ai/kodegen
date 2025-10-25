//! Text-to-Text Generation Capability
//!
//! Models capable of generating text completions from text prompts.

pub mod qwen3_quantized;

// Re-exports for convenience
pub use qwen3_quantized::CandleQwen3QuantizedModel;
