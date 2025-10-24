//! Configuration types for LLaVA vision model

/// Configuration for vision model generation
///
/// Controls sampling behavior for LLaVA vision-language generation.
/// Follows the same pattern as CandleCompletionParams for text-to-text models.
#[derive(Debug, Clone)]
pub struct VisionConfig {
    /// Sampling temperature (0.0 = greedy, >0.0 = sampling)
    pub temperature: f64,
    /// Maximum tokens to generate
    pub max_tokens: Option<usize>,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            max_tokens: None,
        }
    }
}

/// Image processing configuration for LLaVA
#[derive(Debug, Clone, Copy)]
pub(crate) struct ImageProcessingConfig {
    pub image_size: usize,
    pub image_mean: [f32; 3],
    pub image_std: [f32; 3],
}

/// Text generation configuration for LLaVA
#[derive(Debug, Clone, Copy)]
pub(crate) struct GenerationConfig {
    pub temperature: f64,
    pub max_new_tokens: usize,
    pub use_kv_cache: bool,
}
