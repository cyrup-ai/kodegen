//! Model capability utilities for filtering and querying
//!
//! This module provides utility types for working with model capabilities.
//! `ModelCapabilities` is derived from `ModelInfo` (the single source of truth)
//! which deserializes directly from the external models.yaml file.

use serde::{Deserialize, Serialize};

/// Specific capabilities that Candle models can support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandleCapability {
    /// Supports vision/image understanding
    Vision,
    /// Supports function/tool calling
    FunctionCalling,
    /// Supports streaming responses
    Streaming,
    /// Supports fine-tuning
    FineTuning,
    /// Supports batch processing
    BatchProcessing,
    /// Supports real-time processing
    Realtime,
    /// Supports multimodal inputs (text + images, etc.)
    Multimodal,
    /// Supports thinking/reasoning modes
    Thinking,
    /// Supports embedding generation
    Embedding,
    /// Supports code completion
    CodeCompletion,
    /// Supports chat/conversation
    Chat,
    /// Supports instruction following
    InstructionFollowing,
    /// Supports few-shot learning
    FewShotLearning,
    /// Supports zero-shot learning
    ZeroShotLearning,
    /// Supports long context windows
    LongContext,
    /// Supports low-latency inference
    LowLatency,
    /// Supports high-throughput inference
    HighThroughput,
    /// Supports model quantization
    Quantization,
    /// Supports model distillation
    Distillation,
    /// Supports model pruning
    Pruning,
}

impl CandleCapability {
    /// Parse a capability string (case-insensitive, supports `snake_case` and kebab-case)
    #[must_use]
    pub fn from_string(s: &str) -> Option<Self> {
        let normalized = s.to_lowercase().replace('-', "_");
        match normalized.as_str() {
            "vision" => Some(Self::Vision),
            "function_calling" => Some(Self::FunctionCalling),
            "streaming" => Some(Self::Streaming),
            "fine_tuning" => Some(Self::FineTuning),
            "batch_processing" => Some(Self::BatchProcessing),
            "realtime" => Some(Self::Realtime),
            "multimodal" => Some(Self::Multimodal),
            "thinking" => Some(Self::Thinking),
            "embedding" => Some(Self::Embedding),
            "code_completion" => Some(Self::CodeCompletion),
            "chat" => Some(Self::Chat),
            "instruction_following" => Some(Self::InstructionFollowing),
            "few_shot_learning" => Some(Self::FewShotLearning),
            "zero_shot_learning" => Some(Self::ZeroShotLearning),
            "long_context" => Some(Self::LongContext),
            "low_latency" => Some(Self::LowLatency),
            "high_throughput" => Some(Self::HighThroughput),
            "quantization" => Some(Self::Quantization),
            "distillation" => Some(Self::Distillation),
            "pruning" => Some(Self::Pruning),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// Model capability flags using bitflags for zero-allocation capability checks
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct ModelCapabilityFlags: u32 {
        const VISION = 1 << 0;
        const FUNCTION_CALLING = 1 << 1;
        const STREAMING = 1 << 2;
        const FINE_TUNING = 1 << 3;
        const BATCH_PROCESSING = 1 << 4;
        const REALTIME = 1 << 5;
        const MULTIMODAL = 1 << 6;
        const THINKING = 1 << 7;
        const EMBEDDING = 1 << 8;
        const CODE_COMPLETION = 1 << 9;
        const CHAT = 1 << 10;
        const INSTRUCTION_FOLLOWING = 1 << 11;
        const FEW_SHOT_LEARNING = 1 << 12;
        const ZERO_SHOT_LEARNING = 1 << 13;
        const LONG_CONTEXT = 1 << 14;
        const LOW_LATENCY = 1 << 15;
        const HIGH_THROUGHPUT = 1 << 16;
        const QUANTIZATION = 1 << 17;
        const DISTILLATION = 1 << 18;
        const PRUNING = 1 << 19;
    }
}

impl Default for ModelCapabilityFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// Candle model capability flags for filtering and selection
///
/// This is a utility struct derived from `CandleModelInfo` for capability-based filtering.
/// `CandleModelInfo` (which deserializes from the external models.yaml) is the single source of truth.
/// Use `CandleModelInfo::to_capabilities()` to create this struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CandleModelCapabilities {
    /// Capability flags
    pub flags: ModelCapabilityFlags,
}

impl CandleModelCapabilities {
    /// Create a new `ModelCapabilities` with all capabilities disabled
    ///
    /// **NOTE**: In most cases, you should use `ModelInfo::to_capabilities()` instead
    /// of creating `ModelCapabilities` directly, since `ModelInfo` is the single source
    /// of truth that deserializes from the external models.yaml file.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable a specific capability
    #[must_use]
    pub fn with_capability(mut self, capability: CandleCapability) -> Self {
        self.set_capability(capability, true);
        self
    }

    /// Disable a specific capability
    #[must_use]
    pub fn without_capability(mut self, capability: CandleCapability) -> Self {
        self.set_capability(capability, false);
        self
    }

    /// Set a specific capability
    pub fn set_capability(&mut self, capability: CandleCapability, enabled: bool) {
        let flag = Self::capability_to_flag(capability);
        self.flags.set(flag, enabled);
    }

    /// Convert capability enum to flag
    const fn capability_to_flag(capability: CandleCapability) -> ModelCapabilityFlags {
        match capability {
            CandleCapability::Vision => ModelCapabilityFlags::VISION,
            CandleCapability::FunctionCalling => ModelCapabilityFlags::FUNCTION_CALLING,
            CandleCapability::Streaming => ModelCapabilityFlags::STREAMING,
            CandleCapability::FineTuning => ModelCapabilityFlags::FINE_TUNING,
            CandleCapability::BatchProcessing => ModelCapabilityFlags::BATCH_PROCESSING,
            CandleCapability::Realtime => ModelCapabilityFlags::REALTIME,
            CandleCapability::Multimodal => ModelCapabilityFlags::MULTIMODAL,
            CandleCapability::Thinking => ModelCapabilityFlags::THINKING,
            CandleCapability::Embedding => ModelCapabilityFlags::EMBEDDING,
            CandleCapability::CodeCompletion => ModelCapabilityFlags::CODE_COMPLETION,
            CandleCapability::Chat => ModelCapabilityFlags::CHAT,
            CandleCapability::InstructionFollowing => ModelCapabilityFlags::INSTRUCTION_FOLLOWING,
            CandleCapability::FewShotLearning => ModelCapabilityFlags::FEW_SHOT_LEARNING,
            CandleCapability::ZeroShotLearning => ModelCapabilityFlags::ZERO_SHOT_LEARNING,
            CandleCapability::LongContext => ModelCapabilityFlags::LONG_CONTEXT,
            CandleCapability::LowLatency => ModelCapabilityFlags::LOW_LATENCY,
            CandleCapability::HighThroughput => ModelCapabilityFlags::HIGH_THROUGHPUT,
            CandleCapability::Quantization => ModelCapabilityFlags::QUANTIZATION,
            CandleCapability::Distillation => ModelCapabilityFlags::DISTILLATION,
            CandleCapability::Pruning => ModelCapabilityFlags::PRUNING,
        }
    }

    /// Check if a specific capability is enabled
    #[must_use]
    pub fn has_capability(&self, capability: CandleCapability) -> bool {
        let flag = Self::capability_to_flag(capability);
        self.flags.contains(flag)
    }

    /// Check if all specified capabilities are enabled
    #[must_use]
    pub fn has_all_capabilities(&self, capabilities: &[CandleCapability]) -> bool {
        capabilities.iter().all(|&cap| self.has_capability(cap))
    }

    /// Check if any of the specified capabilities are enabled
    #[must_use]
    pub fn has_any_capability(&self, capabilities: &[CandleCapability]) -> bool {
        capabilities.iter().any(|&cap| self.has_capability(cap))
    }

    /// Get an iterator over all enabled capabilities
    pub fn enabled_capabilities(&self) -> impl Iterator<Item = CandleCapability> + '_ {
        use CandleCapability::{
            BatchProcessing, Chat, CodeCompletion, Distillation, Embedding, FewShotLearning,
            FineTuning, FunctionCalling, HighThroughput, InstructionFollowing, LongContext,
            LowLatency, Multimodal, Pruning, Quantization, Realtime, Streaming, Thinking, Vision,
            ZeroShotLearning,
        };
        [
            Vision,
            FunctionCalling,
            Streaming,
            FineTuning,
            BatchProcessing,
            Realtime,
            Multimodal,
            Thinking,
            Embedding,
            CodeCompletion,
            Chat,
            InstructionFollowing,
            FewShotLearning,
            ZeroShotLearning,
            LongContext,
            LowLatency,
            HighThroughput,
            Quantization,
            Distillation,
            Pruning,
        ]
        .iter()
        .filter(move |&&capability| self.has_capability(capability))
        .copied()
    }

    /// Get all enabled capabilities as a vector
    #[must_use]
    pub fn to_vec(&self) -> Vec<CandleCapability> {
        self.enabled_capabilities().collect()
    }
}

/// Candle model performance characteristics
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CandleModelPerformance {
    /// Tokens per second for input processing
    pub input_tokens_per_second: f32,
    /// Tokens per second for output generation
    pub output_tokens_per_second: f32,
    /// Latency in milliseconds for the first token
    pub first_token_latency_ms: f32,
    /// Latency in milliseconds per token
    pub per_token_latency_ms: f32,
    /// Memory usage in MB
    pub memory_usage_mb: f32,
    /// GPU memory usage in MB (if applicable)
    pub gpu_memory_usage_mb: Option<f32>,
    /// Number of parameters in billions
    pub parameter_count_billions: f32,
    /// Floating-point operations per token
    pub flops_per_token: Option<u64>,
}

impl Default for CandleModelPerformance {
    fn default() -> Self {
        Self {
            input_tokens_per_second: 0.0,
            output_tokens_per_second: 0.0,
            first_token_latency_ms: 0.0,
            per_token_latency_ms: 0.0,
            memory_usage_mb: 0.0,
            gpu_memory_usage_mb: None,
            parameter_count_billions: 0.0,
            flops_per_token: None,
        }
    }
}

/// Common use cases for Candle model selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandleUseCase {
    /// General chat/conversation
    Chat,
    /// Code generation and completion
    CodeGeneration,
    /// Text summarization
    Summarization,
    /// Text classification
    Classification,
    /// Named entity recognition
    NamedEntityRecognition,
    /// Question answering
    QuestionAnswering,
    /// Text embedding generation
    Embedding,
    /// Text generation
    TextGeneration,
    /// Translation between languages
    Translation,
    /// Sentiment analysis
    SentimentAnalysis,
    /// Text-to-Speech
    TextToSpeech,
    /// Speech-to-Text
    SpeechToText,
    /// Image generation
    ImageGeneration,
    /// Image classification
    ImageClassification,
    /// Object detection
    ObjectDetection,
    /// Video understanding
    VideoUnderstanding,
    /// Audio processing
    AudioProcessing,
    /// Multimodal tasks
    Multimodal,
    /// Reasoning/thinking tasks
    Reasoning,
    /// Few-shot learning
    FewShotLearning,
    /// Zero-shot learning
    ZeroShotLearning,
}
