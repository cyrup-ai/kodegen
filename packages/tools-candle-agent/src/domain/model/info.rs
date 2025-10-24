//! Model information types and utilities

// Removed unused import: std::borrow::Cow
// Removed unused import: std::fmt
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

// Removed unused import: smallvec::SmallVec
use crate::domain::model::error::{CandleModelError, CandleResult};

/// AI model provider organizations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandleProvider {
    /// Moonshot AI (Kimi models)
    #[serde(rename = "moonshot-ai")]
    MoonshotAI,
    /// Microsoft (Phi models)
    #[serde(rename = "microsoft")]
    Microsoft,
    /// Alibaba NLP (GTE, Qwen models)
    #[serde(rename = "alibaba-nlp")]
    AlibabaNLP,
    /// Stability AI (Stable Diffusion models)
    #[serde(rename = "stability-ai")]
    StabilityAI,
    /// Black Forest Labs (FLUX models)
    #[serde(rename = "black-forest-labs")]
    BlackForestLabs,
    /// `OpenAI` (CLIP models)
    #[serde(rename = "openai")]
    OpenAI,
    /// Sentence Transformers (BERT models)
    #[serde(rename = "sentence-transformers")]
    SentenceTransformers,
    /// Jina AI (Jina-BERT models)
    #[serde(rename = "jina-ai")]
    JinaAI,
    /// NVIDIA (NV-Embed models)
    #[serde(rename = "nvidia")]
    Nvidia,
    /// Dunzhang (Stella models)
    #[serde(rename = "dunzhang")]
    Dunzhang,
    /// `LLaVA` HF (`LLaVA` models)
    #[serde(rename = "llava-hf")]
    LLaVAHF,
    /// Unsloth (Quantized models)
    #[serde(rename = "unsloth")]
    Unsloth,
    /// LAION (Large-scale AI Open Network)
    #[serde(rename = "laion")]
    LAION,
    /// Google (T5, other models)
    #[serde(rename = "google")]
    Google,
    /// Community contributors on `HuggingFace`
    #[serde(rename = "community")]
    Community,
}

impl CandleProvider {
    /// Convert provider to string representation
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            CandleProvider::MoonshotAI => "moonshot-ai",
            CandleProvider::Microsoft => "microsoft",
            CandleProvider::AlibabaNLP => "alibaba-nlp",
            CandleProvider::StabilityAI => "stability-ai",
            CandleProvider::BlackForestLabs => "black-forest-labs",
            CandleProvider::OpenAI => "openai",
            CandleProvider::SentenceTransformers => "sentence-transformers",
            CandleProvider::JinaAI => "jina-ai",
            CandleProvider::Nvidia => "nvidia",
            CandleProvider::Dunzhang => "dunzhang",
            CandleProvider::LLaVAHF => "llava-hf",
            CandleProvider::Unsloth => "unsloth",
            CandleProvider::LAION => "laion",
            CandleProvider::Google => "google",
            CandleProvider::Community => "community",
        }
    }
}

impl std::fmt::Display for CandleProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Core metadata and capabilities for a Candle AI model
///
/// This struct provides a standardized way to represent Candle model capabilities,
/// limitations, and metadata across different providers.
///
/// **IMPORTANT**: This struct deserializes directly from the external models.yaml
/// file curated by sigoden on GitHub. The field names and structure must match
/// that YAML format exactly. `CandleModelInfo` is the single source of truth for model data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[must_use = "CandleModelInfo should be used to make informed decisions about model selection"]
#[allow(clippy::struct_excessive_bools)] // YAML deserialization; bools are independent feature flags from external source
pub struct CandleModelInfo {
    /// The model provider organization
    pub provider: CandleProvider,

    /// The name of the model (e.g., "kimi-k2-instruct", "qwen3-coder-30b")
    pub name: &'static str,

    /// `HuggingFace` registry key (org/model-id)
    pub registry_key: &'static str,

    /// Optional path to quantized model file in format "org/repo/filename.gguf"
    /// If specified, enables cross-repository file loading for quantized variants
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantization_url: Option<&'static str>,

    /// Maximum number of input tokens supported by the model
    pub max_input_tokens: Option<NonZeroU32>,

    /// Maximum number of output tokens that can be generated
    pub max_output_tokens: Option<NonZeroU32>,

    /// Price per 1M input tokens in USD (if known)
    pub input_price: Option<f64>,

    /// Price per 1M output tokens in USD (if known)
    pub output_price: Option<f64>,

    /// Whether the model supports image/video input (multimodal)
    #[serde(default)]
    pub supports_vision: bool,

    /// Whether the model supports function calling/tool use
    #[serde(default)]
    pub supports_function_calling: bool,

    /// Whether the model supports streaming responses
    #[serde(default)]
    pub supports_streaming: bool,

    /// Whether the model supports embeddings
    #[serde(default)]
    pub supports_embeddings: bool,

    /// Whether the model requires `max_tokens` to be specified
    #[serde(default)]
    pub requires_max_tokens: bool,

    /// Whether the model supports thinking/reasoning capabilities
    #[serde(default)]
    pub supports_thinking: bool,

    /// Optimal thinking budget for this model in tokens (if applicable)
    pub optimal_thinking_budget: Option<u32>,

    /// System prompt prefix for this model (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt_prefix: Option<String>,

    /// Real name of the model (if different from name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub real_name: Option<String>,

    /// Model type (e.g., "embedding" for embedding models)
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "type")]
    pub model_type: Option<String>,

    /// Short CLI identifier for model selection (e.g., "kimi-k2", "qwen-coder")
    pub model_id: &'static str,

    /// Model quantization format (e.g., "`Q4_0`", "`Q5_0`", "F16", "none")
    pub quantization: &'static str,

    /// Patch configuration for API requests
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<serde_json::Value>,

    // === EMBEDDING MODELS ===
    /// Output dimension for embedding models
    pub embedding_dimension: Option<u32>,

    /// Vocabulary size for tokenizer
    pub vocab_size: Option<u32>,

    // === VISION MODELS ===
    /// Input image size (e.g., 336 for `LLaVA`, CLIP)
    pub image_size: Option<u32>,

    /// Image normalization mean (`ImageNet`: [0.485, 0.456, 0.406])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_mean: Option<[f32; 3]>,

    /// Image normalization std (`ImageNet`: [0.229, 0.224, 0.225])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_std: Option<[f32; 3]>,

    // === GENERATION DEFAULTS ===
    /// Default sampling temperature
    pub default_temperature: Option<f64>,

    /// Default top-k sampling
    pub default_top_k: Option<u32>,

    /// Default top-p (nucleus) sampling
    pub default_top_p: Option<f64>,

    // === OPTIMIZATION FLAGS ===
    /// Whether model supports KV caching
    #[serde(default)]
    pub supports_kv_cache: bool,

    /// Whether model supports flash attention
    #[serde(default)]
    pub supports_flash_attention: bool,

    /// Whether model should use BF16 precision
    #[serde(default)]
    pub use_bf16: bool,

    // === DIFFUSION MODELS ===
    /// Default number of diffusion steps
    pub default_steps: Option<u32>,

    /// Default guidance scale for classifier-free guidance
    pub default_guidance_scale: Option<f64>,

    /// Time shift parameter for diffusion
    pub time_shift: Option<f64>,

    /// Estimated memory allocation per worker in MB
    /// Used by pool for dynamic worker limit calculations (80% memory constraint)
    pub est_memory_allocation_mb: usize,
}

impl CandleModelInfo {
    /// Get the full model identifier as "provider:name"
    #[inline]
    #[must_use]
    pub fn id(&self) -> &'static str {
        self.name
    }

    /// Get the provider
    #[inline]
    #[must_use]
    pub fn provider(&self) -> CandleProvider {
        self.provider
    }

    /// Get the registry key (org/model-id for `HuggingFace`)
    #[inline]
    #[must_use]
    pub fn registry_key(&self) -> &'static str {
        self.registry_key
    }

    /// Get the provider as a string
    #[inline]
    #[must_use]
    pub fn provider_str(&self) -> &'static str {
        self.provider.as_str()
    }

    /// Get the `HuggingFace` repository URL (computed from `registry_key`)
    #[inline]
    #[must_use]
    pub fn hf_repo_url(&self) -> String {
        format!("https://huggingface.co/{}", self.registry_key)
    }

    /// Get the model name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Check if the model supports vision
    #[inline]
    #[must_use]
    pub fn has_vision(&self) -> bool {
        self.supports_vision
    }

    /// Check if the model supports function calling
    #[inline]
    #[must_use]
    pub fn has_function_calling(&self) -> bool {
        self.supports_function_calling
    }

    /// Check if the model supports streaming
    #[inline]
    #[must_use]
    pub fn has_streaming(&self) -> bool {
        self.supports_streaming
    }

    /// Check if the model supports embeddings
    #[inline]
    #[must_use]
    pub fn has_embeddings(&self) -> bool {
        self.supports_embeddings
    }

    /// Check if the model requires `max_tokens` to be specified
    #[inline]
    #[must_use]
    pub fn requires_max_tokens(&self) -> bool {
        self.requires_max_tokens
    }

    /// Check if the model supports thinking/reasoning
    #[inline]
    #[must_use]
    pub fn has_thinking(&self) -> bool {
        self.supports_thinking
    }

    /// Get the optimal thinking budget if supported
    #[inline]
    #[must_use]
    pub fn thinking_budget(&self) -> Option<u32> {
        self.optimal_thinking_budget
    }

    /// Get the model's short CLI identifier
    #[inline]
    #[must_use]
    pub fn model_id(&self) -> &'static str {
        self.model_id
    }

    /// Get the model's quantization format
    #[inline]
    #[must_use]
    pub fn quantization(&self) -> &'static str {
        self.quantization
    }

    /// Get the price for a given number of input tokens
    #[inline]
    #[must_use]
    pub fn price_for_input(&self, tokens: u32) -> Option<f64> {
        self.input_price
            .map(|price| (price * f64::from(tokens)) / 1_000_000.0)
    }

    /// Get the price for a given number of output tokens
    #[inline]
    #[must_use]
    pub fn price_for_output(&self, tokens: u32) -> Option<f64> {
        self.output_price
            .map(|price| (price * f64::from(tokens)) / 1_000_000.0)
    }

    /// Convert to `CandleModelCapabilities` for filtering and querying
    ///
    /// This creates a `CandleModelCapabilities` struct from this `CandleModelInfo` instance.
    /// `CandleModelInfo` remains the single source of truth from YAML deserialization.
    #[must_use]
    pub fn to_capabilities(&self) -> crate::domain::model::capabilities::CandleModelCapabilities {
        use crate::domain::model::capabilities::ModelCapabilityFlags;

        let mut flags = ModelCapabilityFlags::empty();

        // Set flags based on model info
        if self.supports_vision {
            flags |= ModelCapabilityFlags::VISION | ModelCapabilityFlags::MULTIMODAL;
        }
        if self.supports_function_calling {
            flags |= ModelCapabilityFlags::FUNCTION_CALLING;
        }
        if self.supports_streaming {
            flags |= ModelCapabilityFlags::STREAMING;
        }
        if self.supports_thinking {
            flags |= ModelCapabilityFlags::THINKING;
        }
        if self.supports_embeddings {
            flags |= ModelCapabilityFlags::EMBEDDING;
        }

        // Assume all models support these common capabilities
        flags |= ModelCapabilityFlags::CHAT
            | ModelCapabilityFlags::INSTRUCTION_FOLLOWING
            | ModelCapabilityFlags::FEW_SHOT_LEARNING
            | ModelCapabilityFlags::ZERO_SHOT_LEARNING;

        // Check for long context window
        if self
            .max_input_tokens
            .is_some_and(|tokens| tokens.get() > 32000)
        {
            flags |= ModelCapabilityFlags::LONG_CONTEXT;
        }

        crate::domain::model::capabilities::CandleModelCapabilities { flags }
    }

    /// Validate the model configuration
    ///
    /// # Errors
    ///
    /// Returns `CandleModelError::InvalidConfiguration` if:
    /// - `provider_name` or `name` is empty
    /// - `max_input_tokens` or `max_output_tokens` is 0
    pub fn validate(&self) -> CandleResult<()> {
        if self.registry_key.is_empty() {
            return Err(CandleModelError::InvalidConfiguration(
                "registry_key cannot be empty".into(),
            ));
        }

        if self.name.is_empty() {
            return Err(CandleModelError::InvalidConfiguration(
                "name cannot be empty".into(),
            ));
        }

        if let Some(max_input) = self.max_input_tokens
            && max_input.get() == 0
        {
            return Err(CandleModelError::InvalidConfiguration(
                "max_input_tokens cannot be zero".into(),
            ));
        }

        if let Some(max_output) = self.max_output_tokens
            && max_output.get() == 0
        {
            return Err(CandleModelError::InvalidConfiguration(
                "max_output_tokens cannot be zero".into(),
            ));
        }

        if self.supports_thinking && self.optimal_thinking_budget.is_none() {
            return Err(CandleModelError::InvalidConfiguration(
                "optimal_thinking_budget must be set when supports_thinking is true".into(),
            ));
        }

        Ok(())
    }
}

impl Hash for CandleModelInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.provider.hash(state);
        self.name.hash(state);
    }
}

/// A collection of model information for a specific provider
#[derive(Debug, Clone)]
pub struct CandleProviderModels {
    provider: CandleProvider,
    models: Vec<CandleModelInfo>,
}

impl CandleProviderModels {
    /// Create a new provider model collection
    #[inline]
    #[must_use]
    pub fn new(provider: CandleProvider) -> Self {
        Self {
            provider,
            models: Vec::new(),
        }
    }

    /// Add a model to the collection
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Model provider doesn't match collection provider
    /// - Model with same name already exists in collection
    pub fn add_model(&mut self, model: CandleModelInfo) -> CandleResult<()> {
        if model.provider != self.provider {
            return Err(CandleModelError::InvalidConfiguration(
                "model provider does not match collection provider".into(),
            ));
        }

        if self.models.iter().any(|m| m.name == model.name) {
            return Err(CandleModelError::ModelAlreadyExists {
                provider: self.provider.to_string().into(),
                name: model.name.into(),
            });
        }

        self.models.push(model);
        Ok(())
    }

    /// Get a model by name
    #[inline]
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CandleModelInfo> {
        self.models.iter().find(|m| m.name == name)
    }

    /// Get all models
    #[inline]
    pub fn all(&self) -> &[CandleModelInfo] {
        &self.models
    }

    /// Get the provider
    #[inline]
    #[must_use]
    pub fn provider(&self) -> CandleProvider {
        self.provider
    }
}
