//! Worker thread management for LLaVA model

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use candle_core::Device;
use candle_nn::VarBuilder;
use candle_transformers::models::llava::{LLaVA, config::LLaVAConfig as CandleLLaVAConfig};
use tokenizers::Tokenizer;

use super::config::{GenerationConfig, ImageProcessingConfig};
use super::inference::{LLaVAConfigs, LLaVAModelRefs, process_ask, process_ask_url};
use super::request::LLaVARequest;
use crate::domain::model::traits::CandleModel;

/// Configuration for LLaVA model
pub(crate) struct LLaVAModelConfig {
    pub llava_config: CandleLLaVAConfig,
    pub device: Device,
    pub image_config: ImageProcessingConfig,
    pub gen_config: GenerationConfig,
}

/// Main model structure with lazy thread initialization
#[derive(Debug, Clone)]
pub(crate) struct LLaVAModelCore {
    pub request_tx: Arc<Mutex<Option<mpsc::UnboundedSender<LLaVARequest>>>>,
}

impl LLaVAModelCore {
    pub fn new() -> Self {
        Self {
            request_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Ensure model thread is spawned (lazy initialization)
    ///
    /// Returns sender for communication with model thread.
    /// Thread spawns on first call, subsequent calls return cached sender.
    pub async fn ensure_thread_spawned<M: CandleModel>(
        &self,
        model_info: &M,
    ) -> Result<mpsc::UnboundedSender<LLaVARequest>, Box<dyn std::error::Error + Send + Sync>> {
        // Check if thread already spawned (quick check without holding lock across await)
        {
            let tx_guard = self.request_tx.lock().await;

            if let Some(sender) = tx_guard.as_ref() {
                return Ok(sender.clone());
            }
        } // Lock released here

        // === FIRST USE: Initialize thread ===

        // Step 1: Get model files via huggingface_file() BEFORE spawning
        // This downloads files if needed and returns cached paths
        let tokenizer_path = model_info
            .huggingface_file(model_info.info().registry_key, "tokenizer.json")
            .await?;
        let weights_path = model_info
            .huggingface_file(model_info.info().registry_key, "model.safetensors")
            .await?;
        let config_path = model_info
            .huggingface_file(model_info.info().registry_key, "config.json")
            .await?;

        // Step 2: Load LLaVA config (CandleLLaVAConfig, not our deleted LLaVAConfig!)
        let llava_config: CandleLLaVAConfig = serde_json::from_slice(
            &tokio::fs::read(&config_path)
                .await
                .map_err(|e| format!("Failed to read config: {}", e))?,
        )
        .map_err(|e| format!("Failed to parse config: {}", e))?;

        // Step 3: Create channels for request/response
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (init_tx, mut init_rx) = mpsc::unbounded_channel();

        // Step 4: Determine device
        let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);

        // Step 5: Extract ALL config from ModelInfo (model_info.info())
        // These will be passed to thread as parameters
        let image_size = model_info
            .info()
            .image_size
            .ok_or("image_size not in ModelInfo")? as usize;
        let image_mean = model_info
            .info()
            .image_mean
            .ok_or("image_mean not in ModelInfo")?;
        let image_std = model_info
            .info()
            .image_std
            .ok_or("image_std not in ModelInfo")?;
        let temperature = model_info
            .info()
            .default_temperature
            .ok_or("default_temperature not in ModelInfo")?;
        let max_new_tokens = model_info
            .info()
            .max_output_tokens
            .ok_or("max_output_tokens not in ModelInfo")?
            .get() as usize;
        let use_kv_cache = model_info.info().supports_kv_cache;

        // Step 6: Spawn task for !Send model
        // Note: spawn_blocking is required because model is !Send and cannot use tokio::spawn
        // Model loading also involves blocking I/O (file reads, GPU memory allocation)
        tokio::task::spawn_blocking(move || {
            // Create a new tokio runtime for this blocking thread
            // Use multi_thread with 1 worker to enable spawn_blocking within worker
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = init_tx.send(Err(format!("Failed to create worker runtime: {}", e)));
                    return;
                }
            };

            let local = tokio::task::LocalSet::new();
            rt.block_on(local.run_until(async move {
                // Load tokenizer
                let tokenizer = match Tokenizer::from_file(&tokenizer_path) {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("Tokenizer load failed: {}", e)));
                        return;
                    }
                };

                // Load model weights INSIDE thread
                let vb = match unsafe {
                    VarBuilder::from_mmaped_safetensors(
                        &[weights_path],
                        candle_core::DType::F16,
                        &device,
                    )
                } {
                    Ok(vb) => vb,
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("VarBuilder failed: {}", e)));
                        return;
                    }
                };

                // Load LLaVA model INSIDE thread (this is the non-Send part!)
                let model = match LLaVA::load(vb, &llava_config, None) {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = init_tx.send(Err(format!("Model load failed: {}", e)));
                        return;
                    }
                };

                // Signal successful initialization
                let _ = init_tx.send(Ok(()));

                // Spawn local task for !Send model
                tokio::task::spawn_local(async move {
                    model_task_with_config(
                        model,
                        tokenizer,
                        LLaVAModelConfig {
                            llava_config,
                            device,
                            image_config: ImageProcessingConfig {
                                image_size,
                                image_mean,
                                image_std,
                            },
                            gen_config: GenerationConfig {
                                temperature,
                                max_new_tokens,
                                use_kv_cache,
                            },
                        },
                        request_rx,
                    )
                    .await;
                })
                .await
                .ok();
            }));
        });

        // Step 7: Wait for initialization to complete
        match init_rx.recv().await {
            Some(Ok(())) => {}
            Some(Err(e)) => return Err(e.into()),
            None => return Err("Init channel closed unexpectedly".into()),
        };

        // Step 8: Store sender for future calls
        {
            let mut tx_guard = self.request_tx.lock().await;
            *tx_guard = Some(request_tx.clone());
        }
        Ok(request_tx)
    }
}

/// Async task that processes requests (runs forever until shutdown)
///
/// All config values passed as parameters (from ModelInfo via ensure_thread_spawned)
/// Runs in LocalSet context to handle !Send model
async fn model_task_with_config(
    model: LLaVA,
    tokenizer: Tokenizer,
    config: LLaVAModelConfig,
    mut request_rx: mpsc::UnboundedReceiver<LLaVARequest>,
) {
    let LLaVAModelConfig {
        llava_config,
        device,
        image_config,
        gen_config,
    } = config;

    while let Some(request) = request_rx.recv().await {
        match request {
            LLaVARequest::Ask {
                image_path,
                question,
                config: user_config,
                response_tx,
            } => {
                // Compute effective config from user config or defaults
                let effective_gen_config = if let Some(user_cfg) = user_config {
                    GenerationConfig {
                        temperature: user_cfg.temperature,
                        max_new_tokens: user_cfg.max_tokens.unwrap_or(gen_config.max_new_tokens),
                        use_kv_cache: gen_config.use_kv_cache,
                    }
                } else {
                    gen_config
                };

                let result = process_ask(
                    LLaVAModelRefs {
                        model: &model,
                        tokenizer: &tokenizer,
                        llava_config: &llava_config,
                        device: &device,
                    },
                    &image_path,
                    &question,
                    LLaVAConfigs {
                        image_config,
                        gen_config: effective_gen_config,
                    },
                )
                .await;
                let _ = response_tx.send(result);
            }
            LLaVARequest::AskUrl {
                image_url,
                question,
                config: user_config,
                response_tx,
            } => {
                // Compute effective config from user config or defaults
                let effective_gen_config = if let Some(user_cfg) = user_config {
                    GenerationConfig {
                        temperature: user_cfg.temperature,
                        max_new_tokens: user_cfg.max_tokens.unwrap_or(gen_config.max_new_tokens),
                        use_kv_cache: gen_config.use_kv_cache,
                    }
                } else {
                    gen_config
                };

                let result = process_ask_url(
                    LLaVAModelRefs {
                        model: &model,
                        tokenizer: &tokenizer,
                        llava_config: &llava_config,
                        device: &device,
                    },
                    &image_url,
                    &question,
                    LLaVAConfigs {
                        image_config,
                        gen_config: effective_gen_config,
                    },
                )
                .await;
                let _ = response_tx.send(result);
            }
            LLaVARequest::Shutdown => break,
        }
    }
}
