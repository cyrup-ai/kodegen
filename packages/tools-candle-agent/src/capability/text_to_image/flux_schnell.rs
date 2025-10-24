//! FLUX.1-schnell provider
//!
//! Fast text-to-image generation using FLUX's 4-step diffusion model with dual text encoding
//! (T5-XXL + CLIP-L) for efficient inference.

mod flux_clip_tokenizer;
mod flux_t5_config;
mod flux_t5_tokenizer;

use flux_clip_tokenizer::FluxClipTokenizer;
use flux_t5_config::FluxT5Config;
use flux_t5_tokenizer::FluxT5Tokenizer;

use crate::domain::image_generation::{
    ImageGenerationChunk, ImageGenerationConfig, ImageGenerationModel,
};
use crate::domain::model::CandleModelInfo;
use crate::domain::model::traits::CandleModel;
use candle_core::{DType, Device, Tensor};
use candle_nn::{Module, VarBuilder};
use candle_transformers::models::{
    flux,
    stable_diffusion::clip::{ClipTextTransformer, Config as ClipConfig},
    t5::{Config as T5Config, T5EncoderModel},
};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::{Mutex as TokioMutex, mpsc};
use tokio_stream::Stream;

/// Request types for FLUX model thread communication
enum FluxRequest {
    Generate {
        prompt: String,
        config: ImageGenerationConfig,
        response_tx: mpsc::UnboundedSender<ImageGenerationChunk>,
    },
}

/// FLUX.1-schnell provider for fast 4-step text-to-image generation
///
/// The actual FLUX model runs on a dedicated thread to avoid Send/Sync issues
/// with Candle's Module trait. Communication happens via channels.
/// Thread spawns lazily on first use.
#[derive(Clone, Debug)]
pub struct FluxSchnell {
    request_tx: Arc<TokioMutex<Option<mpsc::UnboundedSender<FluxRequest>>>>,
    device: Arc<TokioMutex<Option<Device>>>,
}

/// T5-XXL encoder with tokenizer
struct T5WithTokenizer {
    t5: T5EncoderModel,
    tokenizer: Tokenizer,
}

/// CLIP encoder with tokenizer
struct ClipWithTokenizer {
    clip: ClipTextTransformer,
    tokenizer: Tokenizer,
}

/// Compare two devices for equality
fn devices_match(a: &Device, b: &Device) -> bool {
    // Use Debug formatting for comparison since Device implements Debug
    // but CudaDevice and MetalDevice don't implement PartialEq
    format!("{:?}", a) == format!("{:?}", b)
}

impl Default for FluxSchnell {
    fn default() -> Self {
        Self::new()
    }
}

impl FluxSchnell {
    #[inline]
    pub fn new() -> Self {
        Self {
            request_tx: Arc::new(TokioMutex::new(None)),
            device: Arc::new(TokioMutex::new(None)),
        }
    }

    /// Ensure model thread is spawned (lazy initialization)
    ///
    /// Returns sender for communication with model thread.
    /// Thread spawns on first call, subsequent calls return cached sender.
    async fn ensure_thread_spawned(
        &self,
        device: &Device,
    ) -> Result<mpsc::UnboundedSender<FluxRequest>, String> {
        // Check if thread already spawned
        {
            let tx_guard = self.request_tx.lock().await;

            if let Some(sender) = tx_guard.as_ref() {
                // Validate device matches worker device
                let device_guard = self.device.lock().await;

                if let Some(worker_device) = device_guard.as_ref()
                    && !devices_match(worker_device, device)
                {
                    return Err(format!(
                        "Worker already initialized with {:?}, cannot use {:?}",
                        worker_device, device
                    ));
                }

                return Ok(sender.clone());
            }
        }

        // === FIRST USE: Initialize thread ===
        // Create channel for requests
        let (request_tx, mut request_rx) = mpsc::unbounded_channel::<FluxRequest>();
        let device_clone = device.clone();

        // Spawn dedicated thread with LocalSet for !Send models
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(e) => {
                    eprintln!("FATAL: Failed to create FLUX worker runtime: {}", e);
                    panic!("Cannot initialize FLUX model without tokio runtime");
                }
            };

            let local = tokio::task::LocalSet::new();
            rt.block_on(local.run_until(async move {
                // Load models ONCE inside this thread
                let models = match Self::load_models(&device_clone).await {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("FLUX model loading failed: {}", e);
                        return;
                    }
                };

                // Process requests using loaded models
                let mut models = models;
                while let Some(req) = request_rx.recv().await {
                    match req {
                        FluxRequest::Generate {
                            prompt,
                            config,
                            response_tx,
                        } => {
                            Self::process_generation(
                                &mut models,
                                &prompt,
                                &config,
                                &device_clone,
                                response_tx,
                            );
                        }
                    }
                }
            }))
        });

        // Cache sender
        {
            let mut tx_guard = self.request_tx.lock().await;
            *tx_guard = Some(request_tx.clone());
        }

        // Cache device
        {
            let mut device_guard = self.device.lock().await;
            *device_guard = Some(device.clone());
        }

        Ok(request_tx)
    }

    /// Load all models once
    async fn load_models(device: &Device) -> Result<FluxModels, String> {
        let dtype = device.bf16_default_to_f32();

        // Create temporary FluxSchnell instance for huggingface_file() access
        let temp_flux = FluxSchnell::new();

        // Download FLUX model files (from main repo)
        let flux_path = temp_flux
            .huggingface_file(temp_flux.info().registry_key, "flux1-schnell.safetensors")
            .await
            .map_err(|e| format!("Failed to download FLUX model: {}", e))?;

        let vae_path = temp_flux
            .huggingface_file(temp_flux.info().registry_key, "ae.safetensors")
            .await
            .map_err(|e| format!("Failed to download VAE: {}", e))?;

        // Download T5 files (from separate repos via helper structs)
        let t5_model_path = FluxT5Config
            .huggingface_file(FluxT5Config.info().registry_key, "model.safetensors")
            .await
            .map_err(|e| format!("Failed to download T5 model: {}", e))?;

        let t5_config_path = FluxT5Config
            .huggingface_file(FluxT5Config.info().registry_key, "config.json")
            .await
            .map_err(|e| format!("Failed to download T5 config: {}", e))?;

        let t5_tokenizer_path = FluxT5Tokenizer
            .huggingface_file(
                FluxT5Tokenizer.info().registry_key,
                "t5-v1_1-xxl.tokenizer.json",
            )
            .await
            .map_err(|e| format!("Failed to download T5 tokenizer: {}", e))?;

        // Download CLIP files
        let clip_model_path = FluxClipTokenizer
            .huggingface_file(FluxClipTokenizer.info().registry_key, "model.safetensors")
            .await
            .map_err(|e| format!("Failed to download CLIP model: {}", e))?;

        let clip_tokenizer_path = FluxClipTokenizer
            .huggingface_file(FluxClipTokenizer.info().registry_key, "tokenizer.json")
            .await
            .map_err(|e| format!("Failed to download CLIP tokenizer: {}", e))?;

        // Load encoders (async methods with spawn_blocking for tokenizers)
        let t5_encoder = T5WithTokenizer::load(
            &t5_model_path,
            &t5_config_path,
            &t5_tokenizer_path,
            dtype,
            device,
        )
        .await?;

        let clip_encoder =
            ClipWithTokenizer::load(&clip_model_path, &clip_tokenizer_path, dtype, device).await?;

        // Load FLUX transformer
        let vb_flux = unsafe {
            VarBuilder::from_mmaped_safetensors(std::slice::from_ref(&flux_path), dtype, device)
                .map_err(|e| format!("FLUX VarBuilder creation failed: {}", e))?
        };
        let flux_transformer = flux::model::Flux::new(&flux::model::Config::schnell(), vb_flux)
            .map_err(|e| format!("FLUX model creation failed: {}", e))?;

        // Load VAE
        let vb_vae = unsafe {
            VarBuilder::from_mmaped_safetensors(std::slice::from_ref(&vae_path), dtype, device)
                .map_err(|e| format!("VAE VarBuilder creation failed: {}", e))?
        };
        let vae =
            flux::autoencoder::AutoEncoder::new(&flux::autoencoder::Config::schnell(), vb_vae)
                .map_err(|e| format!("VAE creation failed: {}", e))?;

        Ok(FluxModels {
            t5_encoder,
            clip_encoder,
            flux_transformer,
            vae,
        })
    }

    /// Process generation request using loaded models
    fn process_generation(
        models: &mut FluxModels,
        prompt: &str,
        config: &ImageGenerationConfig,
        device: &Device,
        response_tx: mpsc::UnboundedSender<ImageGenerationChunk>,
    ) {
        // Set random seed if provided
        if let Some(seed) = config.seed
            && let Err(e) = device.set_seed(seed)
        {
            let _ = response_tx.send(ImageGenerationChunk::Error(format!(
                "Seed setting failed: {}",
                e
            )));
            return;
        }

        // Encode text prompt
        let t5_emb = match models.t5_encoder.encode(prompt, device) {
            Ok(emb) => emb,
            Err(e) => {
                let _ = response_tx.send(ImageGenerationChunk::Error(format!(
                    "T5 encoding failed: {}",
                    e
                )));
                return;
            }
        };

        let clip_emb = match models.clip_encoder.encode(prompt, device) {
            Ok(emb) => emb,
            Err(e) => {
                let _ = response_tx.send(ImageGenerationChunk::Error(format!(
                    "CLIP encoding failed: {}",
                    e
                )));
                return;
            }
        };

        // Generate image
        let image = match generate_flux_image(
            &models.flux_transformer,
            &models.vae,
            &t5_emb,
            &clip_emb,
            config,
            device,
            &response_tx,
        ) {
            Ok(result) => result,
            Err(e) => {
                let _ = response_tx.send(ImageGenerationChunk::Error(format!(
                    "Image generation failed: {}",
                    e
                )));
                return;
            }
        };

        let _ = response_tx.send(ImageGenerationChunk::Complete { image });
    }
}

/// Holds loaded FLUX models
struct FluxModels {
    t5_encoder: T5WithTokenizer,
    clip_encoder: ClipWithTokenizer,
    flux_transformer: flux::model::Flux,
    vae: flux::autoencoder::AutoEncoder,
}

impl ImageGenerationModel for FluxSchnell {
    fn generate(
        &self,
        prompt: &str,
        config: &ImageGenerationConfig,
        device: &Device,
    ) -> Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>> {
        let prompt = prompt.to_string();
        let config = config.clone();
        let device = device.clone();
        let self_clone = self.clone();

        Box::pin(crate::async_stream::spawn_stream(
            move |response_tx| async move {
                // Ensure model thread is spawned
                let request_tx = match self_clone.ensure_thread_spawned(&device).await {
                    Ok(tx) => tx,
                    Err(e) => {
                        let _ = response_tx.send(ImageGenerationChunk::Error(format!(
                            "Failed to spawn model thread: {}",
                            e
                        )));
                        return;
                    }
                };

                // Send generation request to model thread
                if let Err(e) = request_tx.send(FluxRequest::Generate {
                    prompt,
                    config,
                    response_tx,
                }) {
                    // Channel closed, model thread died
                    eprintln!("Failed to send request to model thread: {}", e);
                }
                // response_tx is now owned by model thread, which will send chunks
            },
        ))
    }

    fn registry_key(&self) -> &str {
        "flux.1-schnell"
    }

    fn default_steps(&self) -> usize {
        4
    }
}

/// Loaded FLUX.1-schnell model for pool workers
///
/// Wrapper around FluxSchnell that delegates to the thread-based implementation.
/// Pool creates this once per registry_key and worker loop reuses it for all generate_image() calls.
#[derive(Debug)]
pub struct LoadedFluxSchnell {
    model: FluxSchnell,
}

impl LoadedFluxSchnell {
    /// Load model (FluxSchnell uses lazy initialization)
    pub fn load(_config: &FluxSchnell) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            model: FluxSchnell::new(),
        })
    }
}

impl CandleModel for LoadedFluxSchnell {
    fn info(&self) -> &'static CandleModelInfo {
        &FLUX_SCHNELL_MODEL_INFO
    }
}

impl crate::capability::traits::TextToImageCapable for LoadedFluxSchnell {
    fn generate_image(
        &self,
        prompt: &str,
        config: &ImageGenerationConfig,
        device: &Device,
    ) -> Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>> {
        // Delegate to inner FluxSchnell's ImageGenerationModel trait
        <FluxSchnell as ImageGenerationModel>::generate(&self.model, prompt, config, device)
    }

    fn registry_key(&self) -> &str {
        <FluxSchnell as ImageGenerationModel>::registry_key(&self.model)
    }

    fn default_steps(&self) -> usize {
        <FluxSchnell as ImageGenerationModel>::default_steps(&self.model)
    }
}

/// Generate image using 4-step FLUX denoising
fn generate_flux_image(
    flux_transformer: &flux::model::Flux,
    vae: &flux::autoencoder::AutoEncoder,
    t5_emb: &Tensor,
    clip_emb: &Tensor,
    config: &ImageGenerationConfig,
    device: &Device,
    sender: &mpsc::UnboundedSender<ImageGenerationChunk>,
) -> Result<Tensor, String> {
    // 1. Initialize noise
    let img = flux::sampling::get_noise(1, config.height, config.width, device)
        .map_err(|e| format!("Noise generation failed: {}", e))?
        .to_dtype(t5_emb.dtype())
        .map_err(|e| format!("Noise dtype conversion failed: {}", e))?;

    // 2. Prepare State (packs embeddings + image for diffusion)
    let state = flux::sampling::State::new(t5_emb, clip_emb, &img)
        .map_err(|e| format!("State preparation failed: {}", e))?;

    // 3. Get timestep schedule (4 steps for schnell, no shift)
    let timesteps = flux::sampling::get_schedule(4, None);

    // 4. Track progress through denoising steps
    let total_steps = timesteps.len().saturating_sub(1);
    for (step, _window) in timesteps.windows(2).enumerate() {
        let _ = sender.send(ImageGenerationChunk::Step {
            step,
            total: total_steps,
            latent: state.img.clone(),
        });
    }

    // 5. Run full denoise (guidance = 0.0 for schnell)
    let guidance = 0.0;
    let denoised = flux::sampling::denoise(
        flux_transformer,
        &state.img,
        &state.img_ids,
        &state.txt,
        &state.txt_ids,
        &state.vec,
        &timesteps,
        guidance,
    )
    .map_err(|e| format!("Denoising failed: {}", e))?;

    // 6. Unpack latent patches back to spatial dimensions
    let unpacked = flux::sampling::unpack(&denoised, config.height, config.width)
        .map_err(|e| format!("Unpack failed: {}", e))?;

    // 7. VAE decode to pixel space
    let decoded = vae
        .decode(&unpacked)
        .map_err(|e| format!("VAE decode failed: {}", e))?;

    // 8. Post-process: scale from [-1, 1] to [0, 1]
    let image = ((decoded
        .clamp(-1f32, 1f32)
        .map_err(|e| format!("Clamp failed: {}", e))?
        + 1.0)
        .map_err(|e| format!("Add failed: {}", e))?
        * 0.5)
        .map_err(|e| format!("Scale failed: {}", e))?;

    Ok(image)
}

/// T5-XXL encoder with tokenizer
impl T5WithTokenizer {
    async fn load(
        model_file: &PathBuf,
        config_file: &PathBuf,
        tokenizer_file: &Path,
        dtype: DType,
        device: &Device,
    ) -> Result<Self, String> {
        // Load T5 config
        let config_str = tokio::fs::read_to_string(config_file)
            .await
            .map_err(|e| format!("T5 config read failed: {}", e))?;
        let config: T5Config = serde_json::from_str(&config_str)
            .map_err(|e| format!("T5 config parse failed: {}", e))?;

        // Load T5 model
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_file], dtype, device)
                .map_err(|e| format!("T5 VarBuilder failed: {}", e))?
        };
        let t5 = T5EncoderModel::load(vb, &config)
            .map_err(|e| format!("T5 encoder load failed: {}", e))?;

        // Load T5 tokenizer from provided path using spawn_blocking
        let tokenizer_file_owned = tokenizer_file.to_path_buf();
        let tokenizer =
            tokio::task::spawn_blocking(move || Tokenizer::from_file(tokenizer_file_owned))
                .await
                .map_err(|e| format!("spawn_blocking failed: {}", e))?
                .map_err(|e| format!("T5 tokenizer load failed: {}", e))?;

        Ok(Self { t5, tokenizer })
    }

    fn encode(&mut self, text: &str, device: &Device) -> Result<Tensor, String> {
        // Tokenize and resize to exactly 256 tokens (FLUX requirement)
        let mut tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("T5 tokenization failed: {}", e))?
            .get_ids()
            .to_vec();

        tokens.resize(256, 0);

        let tokens_tensor = Tensor::new(&tokens[..], device)
            .map_err(|e| format!("T5 token tensor creation failed: {}", e))?
            .unsqueeze(0)
            .map_err(|e| format!("T5 unsqueeze failed: {}", e))?;

        self.t5
            .forward(&tokens_tensor)
            .map_err(|e| format!("T5 forward failed: {}", e))
    }
}

/// CLIP encoder with tokenizer
impl ClipWithTokenizer {
    async fn load(
        model_file: &PathBuf,
        tokenizer_file: &Path,
        dtype: DType,
        device: &Device,
    ) -> Result<Self, String> {
        // Load CLIP model
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_file], dtype, device)
                .map_err(|e| format!("CLIP VarBuilder failed: {}", e))?
        };

        let clip_config = ClipConfig::sdxl();

        let clip = ClipTextTransformer::new(vb.pp("text_model"), &clip_config)
            .map_err(|e| format!("CLIP encoder creation failed: {}", e))?;

        // Load CLIP tokenizer from provided path using spawn_blocking
        let tokenizer_file_owned = tokenizer_file.to_path_buf();
        let tokenizer =
            tokio::task::spawn_blocking(move || Tokenizer::from_file(tokenizer_file_owned))
                .await
                .map_err(|e| format!("spawn_blocking failed: {}", e))?
                .map_err(|e| format!("CLIP tokenizer load failed: {}", e))?;

        Ok(Self { clip, tokenizer })
    }

    fn encode(&self, text: &str, device: &Device) -> Result<Tensor, String> {
        // Tokenize with CLIP tokenizer (natural max 77 tokens)
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("CLIP tokenization failed: {}", e))?
            .get_ids()
            .to_vec();

        let tokens_tensor = Tensor::new(&tokens[..], device)
            .map_err(|e| format!("CLIP token tensor creation failed: {}", e))?
            .unsqueeze(0)
            .map_err(|e| format!("CLIP unsqueeze failed: {}", e))?;

        self.clip
            .forward(&tokens_tensor)
            .map_err(|e| format!("CLIP forward failed: {}", e))
    }
}

// Static model info for FLUX Schnell
static FLUX_SCHNELL_MODEL_INFO: CandleModelInfo = CandleModelInfo {
    provider: crate::domain::model::CandleProvider::BlackForestLabs,
    name: "FLUX.1-schnell",
    registry_key: "black-forest-labs/FLUX.1-schnell",
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
    model_id: "flux-schnell",
    quantization: "bf16",
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
    supports_flash_attention: false,
    use_bf16: true,
    default_steps: Some(4),
    default_guidance_scale: Some(0.0),
    time_shift: None,
    est_memory_allocation_mb: 0,
};

impl CandleModel for FluxSchnell {
    fn info(&self) -> &'static CandleModelInfo {
        &FLUX_SCHNELL_MODEL_INFO
    }
}

impl crate::capability::traits::TextToImageCapable for FluxSchnell {
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
