//! Stable Diffusion 3.5 Large Turbo provider
//!
//! Text-to-image generation using SD3.5's MMDiT diffusion model with triple CLIP encoding
//! (CLIP-L + CLIP-G + T5-XXL) for 4-step turbo inference.
//!
//! Implementation uses a dedicated worker thread because SD3.5 models contain !Send trait objects.
//! This follows the same pattern as LLaVA vision model.

use crate::domain::image_generation::{
    ImageGenerationChunk, ImageGenerationConfig, ImageGenerationModel,
};
use crate::domain::model::CandleModelInfo;
use crate::domain::model::traits::CandleModel;
use candle_core::Device;
use once_cell::sync::OnceCell;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::Stream;

mod clip_g_tokenizer;
mod clip_l_tokenizer;
mod t5_config;
mod t5_tokenizer;
mod worker;

use clip_g_tokenizer::ClipGTokenizer;
use clip_l_tokenizer::ClipLTokenizer;
use t5_config::T5ConfigModel;
use t5_tokenizer::T5TokenizerModel;
use worker::SD35WorkerRequest;

/// Global worker thread handle (lazy initialized)
static WORKER: OnceCell<mpsc::UnboundedSender<SD35WorkerRequest>> = OnceCell::new();

/// Stable Diffusion 3.5 Large Turbo provider
///
/// Models run on a dedicated worker thread to avoid Send/Sync issues.
#[derive(Clone, Debug)]
pub struct StableDiffusion35Turbo {}

impl StableDiffusion35Turbo {
    pub fn new() -> Self {
        Self {}
    }

    /// Get or initialize the worker thread
    fn get_worker() -> &'static mpsc::UnboundedSender<SD35WorkerRequest> {
        WORKER.get_or_init(worker::spawn_worker)
    }
}

impl Default for StableDiffusion35Turbo {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageGenerationModel for StableDiffusion35Turbo {
    fn generate(
        &self,
        prompt: &str,
        config: &ImageGenerationConfig,
        device: &Device,
    ) -> Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>> {
        let prompt = prompt.to_string();
        let config = config.clone();
        let device = device.clone();
        let model_self = self.clone();

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Download all required model files
            let clip_g_path = match model_self
                .huggingface_file(
                    model_self.info().registry_key,
                    "text_encoders/clip_g.safetensors",
                )
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "CLIP-G download failed: {}",
                        e
                    )));
                    return;
                }
            };

            let clip_l_path = match model_self
                .huggingface_file(
                    model_self.info().registry_key,
                    "text_encoders/clip_l.safetensors",
                )
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "CLIP-L download failed: {}",
                        e
                    )));
                    return;
                }
            };

            let t5xxl_path = match model_self
                .huggingface_file(
                    model_self.info().registry_key,
                    "text_encoders/t5xxl_fp16.safetensors",
                )
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "T5-XXL download failed: {}",
                        e
                    )));
                    return;
                }
            };

            let mmdit_path = match model_self
                .huggingface_file(
                    model_self.info().registry_key,
                    "sd3.5_large_turbo.safetensors",
                )
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "MMDiT download failed: {}",
                        e
                    )));
                    return;
                }
            };

            // Download tokenizers
            let clip_l_tokenizer_path = match ClipLTokenizer
                .huggingface_file(ClipLTokenizer.info().registry_key, "tokenizer.json")
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "CLIP-L tokenizer download failed: {}",
                        e
                    )));
                    return;
                }
            };

            let clip_g_tokenizer_path = match ClipGTokenizer
                .huggingface_file(ClipGTokenizer.info().registry_key, "tokenizer.json")
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "CLIP-G tokenizer download failed: {}",
                        e
                    )));
                    return;
                }
            };

            let t5_config_path = match T5ConfigModel
                .huggingface_file(T5ConfigModel.info().registry_key, "config.json")
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "T5 config download failed: {}",
                        e
                    )));
                    return;
                }
            };

            let t5_tokenizer_path = match T5TokenizerModel
                .huggingface_file(
                    T5TokenizerModel.info().registry_key,
                    "t5-v1_1-xxl.tokenizer.json",
                )
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "T5 tokenizer download failed: {}",
                        e
                    )));
                    return;
                }
            };

            // Create response channel for streaming results
            let (response_tx, mut response_rx) = mpsc::unbounded_channel();

            // Send request to worker thread
            let worker = Self::get_worker();
            let request = SD35WorkerRequest {
                prompt,
                config,
                device,
                model_info: model_self.info(),
                clip_g_path,
                clip_l_path,
                t5xxl_path,
                mmdit_path,
                clip_l_tokenizer_path,
                clip_g_tokenizer_path,
                t5_config_path,
                t5_tokenizer_path,
                response_tx,
            };

            if let Err(e) = worker.send(request) {
                let _ = tx.send(ImageGenerationChunk::Error(format!(
                    "Failed to send to worker: {}",
                    e
                )));
                return;
            }

            // Forward responses from worker to output stream
            while let Some(chunk) = response_rx.recv().await {
                let _ = tx.send(chunk);
            }
        }))
    }

    fn registry_key(&self) -> &str {
        "stable-diffusion-3.5-large-turbo"
    }

    fn default_steps(&self) -> usize {
        4
    }
}

// Static model info for Stable Diffusion 3.5 Turbo
static SD35_TURBO_MODEL_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::StabilityAI,
    name: "stable-diffusion-3.5-large-turbo",
    registry_key: "stabilityai/stable-diffusion-3.5-large-turbo",
    quantization_url: None,
    max_input_tokens: None,
    max_output_tokens: None,
    input_price: None,
    output_price: None,
    supports_vision: false,
    supports_function_calling: false,
    supports_streaming: true,
    supports_embeddings: false,
    requires_max_tokens: false,
    supports_thinking: false,
    optimal_thinking_budget: None,
    system_prompt_prefix: None,
    real_name: None,
    model_type: None,
    model_id: "sd35-turbo",
    quantization: "fp16",
    patch: None,
    embedding_dimension: None,
    vocab_size: None,
    image_size: None,
    image_mean: None,
    image_std: None,
    default_temperature: None,
    default_top_k: None,
    default_top_p: None,
    supports_kv_cache: false,
    supports_flash_attention: true,
    use_bf16: false,
    default_steps: Some(4),
    default_guidance_scale: Some(3.5),
    time_shift: Some(3.0),
    est_memory_allocation_mb: 0,
};

impl CandleModel for StableDiffusion35Turbo {
    fn info(&self) -> &'static CandleModelInfo {
        &SD35_TURBO_MODEL_INFO
    }
}

impl crate::capability::traits::TextToImageCapable for StableDiffusion35Turbo {
    fn generate_image(
        &self,
        prompt: &str,
        config: &crate::domain::image_generation::ImageGenerationConfig,
        device: &candle_core::Device,
    ) -> Pin<Box<dyn Stream<Item = crate::domain::image_generation::ImageGenerationChunk> + Send>>
    {
        // Delegate to ImageGenerationModel trait
        <Self as ImageGenerationModel>::generate(self, prompt, config, device)
    }

    fn registry_key(&self) -> &str {
        <Self as ImageGenerationModel>::registry_key(self)
    }

    fn default_steps(&self) -> usize {
        <Self as ImageGenerationModel>::default_steps(self)
    }
}
