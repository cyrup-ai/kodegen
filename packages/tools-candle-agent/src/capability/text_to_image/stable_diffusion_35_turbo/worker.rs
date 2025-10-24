//! Worker thread for Stable Diffusion 3.5 generation
//!
//! SD3.5 models contain !Send trait objects, so we run them on a dedicated worker thread
//! with a LocalSet. This is the same pattern used in LLaVA.

use crate::domain::image_generation::{ImageGenerationChunk, ImageGenerationConfig};
use crate::domain::model::CandleModelInfo;
use candle_core::{D, DType, Device, IndexOp, Tensor};
use candle_nn::{Module, VarBuilder};
use candle_transformers::models::{
    flux,
    mmdit::model::{Config as MMDiTConfig, MMDiT},
    stable_diffusion::{
        clip::{ClipTextTransformer, Config as ClipConfig},
        vae::AutoEncoderKL,
    },
    t5::{Config as T5Config, T5EncoderModel},
};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;
use tokio::sync::mpsc;

/// Request to the SD3.5 worker thread
pub struct SD35WorkerRequest {
    pub prompt: String,
    pub config: ImageGenerationConfig,
    pub device: Device,
    pub model_info: &'static CandleModelInfo,
    pub clip_g_path: PathBuf,
    pub clip_l_path: PathBuf,
    pub t5xxl_path: PathBuf,
    pub mmdit_path: PathBuf,
    pub clip_l_tokenizer_path: PathBuf,
    pub clip_g_tokenizer_path: PathBuf,
    pub t5_config_path: PathBuf,
    pub t5_tokenizer_path: PathBuf,
    pub response_tx: mpsc::UnboundedSender<ImageGenerationChunk>,
}

/// CLIP encoder with tokenizer
struct ClipWithTokenizer {
    clip: ClipTextTransformer,
    config: ClipConfig,
    tokenizer: Tokenizer,
    max_tokens: usize,
}

/// T5 encoder with tokenizer
struct T5WithTokenizer {
    t5: T5EncoderModel,
    tokenizer: Tokenizer,
    max_tokens: usize,
}

/// Triple CLIP encoder wrapper
struct TripleClipEncoder {
    clip_l: ClipWithTokenizer,
    clip_g: ClipWithTokenizer,
    clip_g_projection: candle_nn::Linear,
    t5: T5WithTokenizer,
}

/// Spawn the worker thread that processes SD3.5 generation requests
pub fn spawn_worker() -> mpsc::UnboundedSender<SD35WorkerRequest> {
    let (request_tx, mut request_rx) = mpsc::unbounded_channel();

    std::thread::spawn(move || {
        // Create worker runtime with LocalSet support
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => {
                eprintln!("FATAL: Failed to create SD3.5 worker runtime: {}", e);
                panic!("Cannot initialize Stable Diffusion 3.5 model without tokio runtime");
            }
        };

        let local = tokio::task::LocalSet::new();

        rt.block_on(local.run_until(async move {
            // Process requests sequentially
            while let Some(req) = request_rx.recv().await {
                if let Err(e) = process_request(req).await {
                    eprintln!("SD3.5 worker error: {}", e);
                }
            }
        }));
    });

    request_tx
}

/// Process a single generation request
async fn process_request(req: SD35WorkerRequest) -> Result<(), String> {
    let SD35WorkerRequest {
        prompt,
        config,
        device,
        model_info,
        clip_g_path,
        clip_l_path,
        t5xxl_path,
        mmdit_path,
        clip_l_tokenizer_path,
        clip_g_tokenizer_path,
        t5_config_path,
        t5_tokenizer_path,
        response_tx,
    } = req;

    // Set random seed
    if let Some(seed) = config.seed {
        device
            .set_seed(seed)
            .map_err(|e| format!("Seed setting failed: {}", e))?;
    }

    // Load triple CLIP encoders
    let encoder_config = TripleClipConfig {
        clip_g_file: &clip_g_path,
        clip_l_file: &clip_l_path,
        t5xxl_file: &t5xxl_path,
        clip_l_tokenizer_path: &clip_l_tokenizer_path,
        clip_g_tokenizer_path: &clip_g_tokenizer_path,
        t5_config_path: &t5_config_path,
        t5_tokenizer_path: &t5_tokenizer_path,
    };
    let mut triple_clip = TripleClipEncoder::load(encoder_config, &device).await?;

    // Encode prompts
    let (context, y) = triple_clip.encode_prompt(&prompt, &device).await?;

    // Handle negative prompt
    let (context, y) = if let Some(neg_prompt) = &config.negative_prompt {
        let (ctx_uncond, y_uncond) = triple_clip.encode_prompt(neg_prompt, &device).await?;
        (
            Tensor::cat(&[&context, &ctx_uncond], 0)
                .map_err(|e| format!("Context concat failed: {}", e))?,
            Tensor::cat(&[&y, &y_uncond], 0).map_err(|e| format!("Y concat failed: {}", e))?,
        )
    } else {
        (
            Tensor::cat(&[&context, &context], 0)
                .map_err(|e| format!("Context dup failed: {}", e))?,
            Tensor::cat(&[&y, &y], 0).map_err(|e| format!("Y dup failed: {}", e))?,
        )
    };

    // Load MMDiT model
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[&mmdit_path], DType::F16, &device)
            .map_err(|e| format!("VarBuilder failed: {}", e))?
    };

    let use_flash_attn = model_info.supports_flash_attention;
    let mmdit = MMDiT::new(
        &MMDiTConfig::sd3_5_large(),
        use_flash_attn,
        vb.pp("model.diffusion_model"),
    )
    .map_err(|e| format!("MMDiT creation failed: {}", e))?;

    // Load VAE
    let vae_config = candle_transformers::models::stable_diffusion::vae::AutoEncoderKLConfig {
        block_out_channels: vec![128, 256, 512, 512],
        layers_per_block: 2,
        latent_channels: 16,
        norm_num_groups: 32,
        use_quant_conv: false,
        use_post_quant_conv: false,
    };

    let vae = AutoEncoderKL::new(vb.pp("first_stage_model"), 3, 3, vae_config)
        .map_err(|e| format!("VAE creation failed: {}", e))?;

    // Get time_shift from model_info
    let time_shift = model_info
        .time_shift
        .ok_or_else(|| "time_shift not configured".to_string())?;

    // Euler sampling - now safe because we're in LocalSet
    let latent = euler_sample(
        &mmdit,
        &y,
        &context,
        &config,
        time_shift,
        &device,
        &response_tx,
    )
    .await?;

    // VAE decode - now safe because we're in LocalSet
    let image = decode_latent(&vae, &latent).await?;

    let _ = response_tx.send(ImageGenerationChunk::Complete { image });

    Ok(())
}

/// Euler sampling with CFG (runs in LocalSet, can use raw pointers safely)
async fn euler_sample(
    mmdit: &MMDiT,
    y: &Tensor,
    context: &Tensor,
    config: &ImageGenerationConfig,
    time_shift: f64,
    device: &Device,
    sender: &mpsc::UnboundedSender<ImageGenerationChunk>,
) -> Result<Tensor, String> {
    let mut x = flux::sampling::get_noise(1, config.height, config.width, device)
        .map_err(|e| format!("Noise generation failed: {}", e))?
        .to_dtype(DType::F16)
        .map_err(|e| format!("Dtype conversion failed: {}", e))?;

    let sigmas: Vec<f64> = (0..=config.steps)
        .map(|i| i as f64 / config.steps as f64)
        .rev()
        .map(|t| time_snr_shift(time_shift, t))
        .collect();

    // Call model directly - we're in LocalSet so model never leaves this thread
    // This is the same pattern as LLaVA: models are !Send but safe in LocalSet
    for (step, window) in sigmas.windows(2).enumerate() {
        let (s_curr, s_prev) = (window[0], window[1]);
        let timestep = s_curr * 1000.0;

        let noise_pred = mmdit
            .forward(
                &Tensor::cat(&[&x, &x], 0)
                    .map_err(|e| format!("Latent concat failed at step {}: {}", step, e))?,
                &Tensor::full(timestep as f32, (2,), device)
                    .map_err(|e| format!("Timestep tensor failed: {}", e))?
                    .contiguous()
                    .map_err(|e| format!("Contiguous failed: {}", e))?,
                y,
                context,
                None,
            )
            .map_err(|e| format!("MMDiT forward failed at step {}: {}", step, e))?;

        let guidance = apply_cfg(config.guidance_scale, &noise_pred)?;

        let step_delta = (guidance * (s_prev - s_curr))
            .map_err(|e| format!("Guidance scaling failed: {}", e))?;
        x = (x + step_delta)
            .map_err(|e| format!("Latent update failed at step {}: {}", step, e))?;

        let _ = sender.send(ImageGenerationChunk::Step {
            step,
            total: config.steps,
            latent: x.clone(),
        });
    }

    Ok(x)
}

fn time_snr_shift(alpha: f64, t: f64) -> f64 {
    alpha * t / (1.0 + (alpha - 1.0) * t)
}

fn apply_cfg(scale: f64, noise_pred: &Tensor) -> Result<Tensor, String> {
    let cond = noise_pred
        .narrow(0, 0, 1)
        .map_err(|e| format!("Cond narrow failed: {}", e))?;
    let uncond = noise_pred
        .narrow(0, 1, 1)
        .map_err(|e| format!("Uncond narrow failed: {}", e))?;

    ((scale * cond).map_err(|e| format!("Cond scale failed: {}", e))?
        - ((scale - 1.0) * uncond).map_err(|e| format!("Uncond scale failed: {}", e))?)
    .map_err(|e| format!("CFG subtraction failed: {}", e))
}

async fn decode_latent(vae: &AutoEncoderKL, latent: &Tensor) -> Result<Tensor, String> {
    let latent_scaled =
        ((latent / 1.5305).map_err(|e| format!("Latent division failed: {}", e))? + 0.0609)
            .map_err(|e| format!("Latent offset failed: {}", e))?;

    // Call VAE directly - we're in LocalSet so model never leaves this thread
    let img = vae
        .decode(&latent_scaled)
        .map_err(|e| format!("VAE decode failed: {}", e))?;

    let img = ((img
        .clamp(-1f32, 1f32)
        .map_err(|e| format!("Clamp failed: {}", e))?
        + 1.0)
        .map_err(|e| format!("Add failed: {}", e))?
        * 0.5)
        .map_err(|e| format!("Scale failed: {}", e))?;

    Ok(img)
}

/// Configuration for loading TripleClipEncoder
struct TripleClipConfig<'a> {
    clip_g_file: &'a PathBuf,
    clip_l_file: &'a PathBuf,
    t5xxl_file: &'a PathBuf,
    clip_l_tokenizer_path: &'a Path,
    clip_g_tokenizer_path: &'a Path,
    t5_config_path: &'a PathBuf,
    t5_tokenizer_path: &'a Path,
}

impl TripleClipEncoder {
    async fn load(config: TripleClipConfig<'_>, device: &Device) -> Result<Self, String> {
        let clip_g_file = config.clip_g_file;
        let clip_l_file = config.clip_l_file;
        let t5xxl_file = config.t5xxl_file;
        let clip_l_tokenizer_path = config.clip_l_tokenizer_path;
        let clip_g_tokenizer_path = config.clip_g_tokenizer_path;
        let t5_config_path = config.t5_config_path;
        let t5_tokenizer_path = config.t5_tokenizer_path;
        let vb_clip_l = unsafe {
            VarBuilder::from_mmaped_safetensors(&[clip_l_file], DType::F16, device)
                .map_err(|e| format!("CLIP-L VarBuilder failed: {}", e))?
        };
        let clip_l =
            ClipWithTokenizer::new(vb_clip_l, ClipConfig::sdxl(), clip_l_tokenizer_path, 77)
                .await?;

        let vb_clip_g = unsafe {
            VarBuilder::from_mmaped_safetensors(&[clip_g_file], DType::F16, device)
                .map_err(|e| format!("CLIP-G VarBuilder failed: {}", e))?
        };
        let clip_g = ClipWithTokenizer::new(
            vb_clip_g.clone(),
            ClipConfig::sdxl2(),
            clip_g_tokenizer_path,
            77,
        )
        .await?;

        let clip_g_projection =
            candle_nn::linear_no_bias(1280, 1280, vb_clip_g.pp("text_projection"))
                .map_err(|e| format!("Text projection creation failed: {}", e))?;

        let vb_t5 = unsafe {
            VarBuilder::from_mmaped_safetensors(&[t5xxl_file], DType::F16, device)
                .map_err(|e| format!("T5 VarBuilder failed: {}", e))?
        };
        let t5 = T5WithTokenizer::new(vb_t5, t5_config_path, t5_tokenizer_path, 77).await?;

        Ok(Self {
            clip_l,
            clip_g,
            clip_g_projection,
            t5,
        })
    }

    async fn encode_prompt(
        &mut self,
        prompt: &str,
        device: &Device,
    ) -> Result<(Tensor, Tensor), String> {
        let (clip_l_emb, clip_l_pooled) = self.clip_l.encode(prompt, device).await?;
        let (clip_g_emb, clip_g_pooled) = self.clip_g.encode(prompt, device).await?;

        let clip_g_pooled_proj = self
            .clip_g_projection
            .forward(
                &clip_g_pooled
                    .unsqueeze(0)
                    .map_err(|e| format!("Unsqueeze failed: {}", e))?,
            )
            .map_err(|e| format!("Projection forward failed: {}", e))?
            .squeeze(0)
            .map_err(|e| format!("Squeeze failed: {}", e))?;

        let y = Tensor::cat(&[&clip_l_pooled, &clip_g_pooled_proj], 0)
            .map_err(|e| format!("Y concatenation failed: {}", e))?
            .unsqueeze(0)
            .map_err(|e| format!("Y unsqueeze failed: {}", e))?;

        let clip_concat = Tensor::cat(&[&clip_l_emb, &clip_g_emb], D::Minus1)
            .map_err(|e| format!("CLIP concatenation failed: {}", e))?
            .pad_with_zeros(D::Minus1, 0, 2048)
            .map_err(|e| format!("CLIP padding failed: {}", e))?;

        let t5_emb = self
            .t5
            .encode(prompt, device)
            .await?
            .to_dtype(DType::F16)
            .map_err(|e| format!("T5 dtype conversion failed: {}", e))?;

        let context = Tensor::cat(&[&clip_concat, &t5_emb], D::Minus2)
            .map_err(|e| format!("Context concatenation failed: {}", e))?;

        Ok((context, y))
    }
}

impl ClipWithTokenizer {
    async fn new(
        vb: VarBuilder<'_>,
        config: ClipConfig,
        tokenizer_path: &Path,
        max_tokens: usize,
    ) -> Result<Self, String> {
        let clip = ClipTextTransformer::new(vb, &config)
            .map_err(|e| format!("CLIP creation failed: {}", e))?;

        let tokenizer_path_owned = tokenizer_path.to_path_buf();
        let tokenizer =
            tokio::task::spawn_blocking(move || Tokenizer::from_file(tokenizer_path_owned))
                .await
                .map_err(|e| format!("spawn_blocking failed: {}", e))?
                .map_err(|e| format!("Tokenizer load failed: {}", e))?;

        Ok(Self {
            clip,
            tokenizer,
            max_tokens,
            config,
        })
    }

    async fn encode(&self, text: &str, device: &Device) -> Result<(Tensor, Tensor), String> {
        let pad_id = match &self.config.pad_with {
            Some(padding) => *self
                .tokenizer
                .get_vocab(true)
                .get(padding.as_str())
                .ok_or_else(|| "Failed to tokenize CLIP padding".to_string())?,
            None => *self
                .tokenizer
                .get_vocab(true)
                .get("<|endoftext|>")
                .ok_or_else(|| "Failed to tokenize CLIP end-of-text".to_string())?,
        };

        let mut tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("Tokenization failed: {}", e))?
            .get_ids()
            .to_vec();

        let eos_pos = tokens.len() - 1;

        while tokens.len() < self.max_tokens {
            tokens.push(pad_id);
        }

        // Call CLIP directly - we're in LocalSet so model never leaves this thread
        let tokens_tensor = Tensor::new(&tokens[..], device)
            .map_err(|e| format!("Token tensor creation failed: {}", e))?
            .unsqueeze(0)
            .map_err(|e| format!("Unsqueeze failed: {}", e))?;

        let (emb, emb_penultimate) = self
            .clip
            .forward_until_encoder_layer(&tokens_tensor, usize::MAX, -2)
            .map_err(|e| format!("CLIP forward failed: {}", e))?;

        let pooled = emb
            .i((0, eos_pos, ..))
            .map_err(|e| format!("Pooled extraction failed: {}", e))?;

        Ok((emb_penultimate, pooled))
    }
}

impl T5WithTokenizer {
    async fn new(
        vb: VarBuilder<'_>,
        config_path: &PathBuf,
        tokenizer_path: &Path,
        max_tokens: usize,
    ) -> Result<Self, String> {
        let config_str = tokio::fs::read_to_string(config_path)
            .await
            .map_err(|e| format!("T5 config read failed: {}", e))?;
        let config: T5Config = serde_json::from_str(&config_str)
            .map_err(|e| format!("T5 config parse failed: {}", e))?;

        let t5 = T5EncoderModel::load(vb, &config)
            .map_err(|e| format!("T5 model load failed: {}", e))?;

        let tokenizer_path_owned = tokenizer_path.to_path_buf();
        let tokenizer =
            tokio::task::spawn_blocking(move || Tokenizer::from_file(tokenizer_path_owned))
                .await
                .map_err(|e| format!("spawn_blocking failed: {}", e))?
                .map_err(|e| format!("T5 tokenizer load failed: {}", e))?;

        Ok(Self {
            t5,
            tokenizer,
            max_tokens,
        })
    }

    async fn encode(&mut self, text: &str, device: &Device) -> Result<Tensor, String> {
        let mut tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("T5 tokenization failed: {}", e))?
            .get_ids()
            .to_vec();

        tokens.resize(self.max_tokens, 0);

        // Call T5 directly - we're in LocalSet so model never leaves this thread
        let tokens_tensor = Tensor::new(&tokens[..], device)
            .map_err(|e| format!("T5 token tensor failed: {}", e))?
            .unsqueeze(0)
            .map_err(|e| format!("T5 unsqueeze failed: {}", e))?;

        self.t5
            .forward_dt(&tokens_tensor, Some(DType::F32))
            .map_err(|e| format!("T5 forward failed: {}", e))
    }
}
