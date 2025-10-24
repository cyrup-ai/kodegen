//! LLaVA (Large Language and Vision Assistant) provider
//!
//! This module wraps Candle's unified LLaVA model for vision-language understanding.
//! Supports visual question answering, image description, and multi-turn conversations.

use std::num::NonZeroU32;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::Stream;

use crate::domain::context::CandleStringChunk;
use crate::domain::model::{CandleModelInfo, CandleProvider, traits::CandleModel};

mod builder;
mod config;
mod inference;
mod request;
mod sampler;
mod tokenizer;
mod worker;

pub use builder::VisionQueryBuilder;
pub use config::VisionConfig;

use request::LLaVARequest;
use worker::LLaVAModelCore;

/// LLaVA vision-language provider
///
/// Wraps Candle's unified LLaVA model (CLIP vision + LLaMA language)
/// for image understanding and visual question answering.
///
/// The actual LLaVA model runs on a dedicated thread to avoid Send/Sync issues
/// with Candle's Module trait. Communication happens via channels.
/// Thread spawns lazily on first use.
#[derive(Debug, Clone)]
pub struct LLaVAModel {
    core: LLaVAModelCore,
}

// Static model info for LLaVA 1.5
pub static LLAVA_MODEL_INFO: CandleModelInfo = CandleModelInfo {
    provider: CandleProvider::LLaVAHF,
    name: "llava-1.5-7b-hf",
    registry_key: "llava-hf/llava-1.5-7b-hf",
    quantization_url: None,
    max_input_tokens: NonZeroU32::new(4096),
    max_output_tokens: NonZeroU32::new(512),
    input_price: None, // Local model - no pricing
    output_price: None,
    supports_vision: true,
    supports_function_calling: false,
    supports_streaming: true,
    supports_embeddings: false,
    requires_max_tokens: false,
    supports_thinking: false,
    optimal_thinking_budget: None,
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "llava",
    quantization: "F16",
    patch: None,
    embedding_dimension: None,
    vocab_size: None,
    image_size: Some(336),
    image_mean: Some([0.48145466, 0.4578275, 0.40821073]),
    image_std: Some([0.26862954, 0.2613026, 0.2757771]),
    default_temperature: Some(0.2),
    default_top_k: None,
    default_top_p: None,
    supports_kv_cache: true,
    supports_flash_attention: false,
    use_bf16: false,
    default_steps: None,
    default_guidance_scale: None,
    time_shift: None,
    est_memory_allocation_mb: 0,
};

impl LLaVAModel {
    /// Create new LLaVA model (lazy initialization)
    ///
    /// Thread spawns on first describe_image() or describe_url() call.
    /// All configuration comes from LLAVA_MODEL_INFO.
    pub fn new() -> Self {
        Self {
            core: LLaVAModelCore::new(),
        }
    }

    /// ONLY public entry point for vision generation
    ///
    /// Builder pattern is the ONLY public API.
    /// Direct describe_image/describe_url are PRIVATE.
    ///
    /// # Example
    /// ```rust
    /// let model = LLaVAModel::new();
    /// model.query()
    ///     .temperature(0.7)
    ///     .max_tokens(256)
    ///     .describe_image("image.jpg", "what is this?")
    ///     .await;
    /// ```
    pub fn query(&self) -> VisionQueryBuilder {
        VisionQueryBuilder::new(self.clone())
    }

    /// PRIVATE: Internal implementation for describe_image
    ///
    /// Called by VisionQueryBuilder with user-configured parameters.
    /// Thread spawns lazily on first call.
    pub(crate) async fn describe_image_internal(
        &self,
        image_path: &str,
        query: &str,
        config: VisionConfig,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        // Ensure thread is spawned (lazy initialization)
        let sender = match self.core.ensure_thread_spawned(self).await {
            Ok(s) => s,
            Err(e) => {
                return Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                    let _ = tx.send(CandleStringChunk::text(format!("Error: {}", e)));
                }));
            }
        };

        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        if let Err(e) = sender.send(LLaVARequest::Ask {
            image_path: image_path.to_string(),
            question: query.to_string(),
            config: Some(config),
            response_tx,
        }) {
            return Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                let _ = tx.send(CandleStringChunk::text(format!("Error: {}", e)));
            }));
        }

        match response_rx.recv().await {
            Some(Ok(text)) => Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                let _ = tx.send(CandleStringChunk::text(text));
            })),
            Some(Err(e)) => Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                let _ = tx.send(CandleStringChunk::text(format!("Error: {}", e)));
            })),
            None => Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                let _ = tx.send(CandleStringChunk::text(
                    "Error: Failed to receive response".to_string(),
                ));
            })),
        }
    }

    /// PRIVATE: Internal implementation for describe_url
    ///
    /// Called by VisionQueryBuilder with user-configured parameters.
    /// Thread spawns lazily on first call.
    pub(crate) async fn describe_url_internal(
        &self,
        image_url: &str,
        query: &str,
        config: VisionConfig,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        // Ensure thread is spawned (lazy initialization)
        let sender = match self.core.ensure_thread_spawned(self).await {
            Ok(s) => s,
            Err(e) => {
                return Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                    let _ = tx.send(CandleStringChunk::text(format!("Error: {}", e)));
                }));
            }
        };

        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        if let Err(e) = sender.send(LLaVARequest::AskUrl {
            image_url: image_url.to_string(),
            question: query.to_string(),
            config: Some(config),
            response_tx,
        }) {
            return Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                let _ = tx.send(CandleStringChunk::text(format!("Error: {}", e)));
            }));
        }

        match response_rx.recv().await {
            Some(Ok(text)) => Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                let _ = tx.send(CandleStringChunk::text(text));
            })),
            Some(Err(e)) => Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                let _ = tx.send(CandleStringChunk::text(format!("Error: {}", e)));
            })),
            None => Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
                let _ = tx.send(CandleStringChunk::text(
                    "Error: Failed to receive response".to_string(),
                ));
            })),
        }
    }

    /// Stream chat responses token by token
    ///
    /// **NOTE**: Due to Candle's LLaVA containing non-Send trait objects,
    /// streaming happens after full generation. Returns buffered tokio stream.
    ///
    /// For true streaming, await the entire response then iterate the stream.
    pub fn stream_chat(
        &self,
        image_path: &str,
        question: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        let model = self.clone();
        let image_path = image_path.to_string();
        let question = question.to_string();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Call internal method with default config
            let stream = model
                .describe_image_internal(&image_path, &question, VisionConfig::default())
                .await;
            use tokio_stream::StreamExt;
            tokio::pin!(stream);
            while let Some(chunk) = stream.next().await {
                let _ = tx.send(chunk);
            }
        }))
    }
}

impl CandleModel for LLaVAModel {
    fn info(&self) -> &'static CandleModelInfo {
        &LLAVA_MODEL_INFO
    }
}

impl crate::capability::traits::VisionCapable for LLaVAModel {
    fn describe_image(
        &self,
        image_path: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        let model = self.clone();
        let image_path = image_path.to_string();
        let query = query.to_string();
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let stream = model
                .describe_image_internal(&image_path, &query, VisionConfig::default())
                .await;
            tokio::pin!(stream);
            use tokio_stream::StreamExt;
            while let Some(chunk) = stream.next().await {
                let _ = tx.send(chunk);
            }
        }))
    }

    fn describe_url(
        &self,
        url: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        let model = self.clone();
        let url = url.to_string();
        let query = query.to_string();
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            let stream = model
                .describe_url_internal(&url, &query, VisionConfig::default())
                .await;
            tokio::pin!(stream);
            use tokio_stream::StreamExt;
            while let Some(chunk) = stream.next().await {
                let _ = tx.send(chunk);
            }
        }))
    }
}

impl Default for LLaVAModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Loaded LLaVA model for pool workers
///
/// Wrapper around LLaVAModel that can be loaded in pool worker threads.
/// Delegates all VisionCapable trait methods to the wrapped model.
#[derive(Debug)]
pub struct LoadedLLaVAModel {
    model: LLaVAModel,
}

impl LoadedLLaVAModel {
    /// Load model from LLaVAModel configuration
    ///
    /// Creates a new LLaVAModel instance in the pool worker thread.
    pub fn load(config: &LLaVAModel) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create new instance - LLaVAModel uses lazy initialization
        let _ = config; // Suppress unused warning
        Ok(Self {
            model: LLaVAModel::new(),
        })
    }
}

impl CandleModel for LoadedLLaVAModel {
    fn info(&self) -> &'static CandleModelInfo {
        self.model.info()
    }
}

impl crate::capability::traits::VisionCapable for LoadedLLaVAModel {
    fn describe_image(
        &self,
        image_path: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        self.model.describe_image(image_path, query)
    }

    fn describe_url(
        &self,
        url: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        self.model.describe_url(url, query)
    }
}
