use candle_core::Device;
use once_cell::sync::Lazy;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::Stream;

use crate::capability::registry::pool::core::memory_governor::AllocationGuard;
use crate::capability::registry::pool::core::types::{
    HealthPing, HealthPong, PendingRequestsGuard, select_worker_power_of_two,
};
use crate::capability::registry::pool::core::{Pool, PoolConfig, PoolError, WorkerHandle};
use crate::capability::traits::TextToImageCapable;
use crate::domain::image_generation::{ImageGenerationChunk, ImageGenerationConfig};

/// Type alias for image generation streaming response sender
type ImageGenerationResponse =
    oneshot::Sender<Result<Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>>, PoolError>>;

/// Request for generate_image() operation (streaming response)
pub struct GenerateImageRequest {
    pub prompt: String,
    pub config: ImageGenerationConfig,
    pub device: Device,
    pub response: ImageGenerationResponse,
}

/// TextToImage-specific worker handle with channel
pub struct TextToImageWorkerHandle {
    pub core: WorkerHandle,
    pub generate_image_tx: mpsc::UnboundedSender<GenerateImageRequest>,
    pub shutdown_tx: mpsc::UnboundedSender<()>,
    pub registry_key: String, // Added to enable cleanup on drop
}

impl crate::capability::registry::pool::core::types::PoolWorkerHandle for TextToImageWorkerHandle {
    fn core(&self) -> &crate::capability::registry::pool::core::WorkerHandle {
        &self.core
    }

    fn core_mut(&mut self) -> &mut crate::capability::registry::pool::core::WorkerHandle {
        &mut self.core
    }

    fn registry_key(&self) -> &str {
        &self.registry_key
    }
}

impl std::ops::Deref for TextToImageWorkerHandle {
    type Target = WorkerHandle;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// Worker loop for TextToImage models
///
/// Processes streaming requests. Worker calls trait method which
/// returns Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>>. Stream is sent back to caller
/// who forwards chunks to end user.
pub async fn text_to_image_worker<T: TextToImageCapable>(
    model: T,
    mut generate_image_rx: mpsc::UnboundedReceiver<GenerateImageRequest>,
    mut shutdown_rx: mpsc::UnboundedReceiver<()>,
    mut health_rx: mpsc::UnboundedReceiver<HealthPing>,
    health_tx: mpsc::UnboundedSender<HealthPong>,
    worker_id: usize,
    state: Arc<AtomicU32>,
) {
    use crate::capability::registry::pool::core::worker_state::WorkerState;
    use std::time::{Duration, SystemTime};

    // Track last activity for idle detection
    let mut last_activity = SystemTime::now();
    let idle_threshold = Duration::from_secs(300); // 5 minutes

    loop {
        // Check for idle timeout (Ready → Idle after 5 minutes of inactivity)
        if let Ok(elapsed) = last_activity.elapsed()
            && elapsed > idle_threshold
        {
            let current_state = WorkerState::from(state.load(std::sync::atomic::Ordering::Acquire));
            if matches!(current_state, WorkerState::Ready) {
                state.store(
                    WorkerState::Idle as u32,
                    std::sync::atomic::Ordering::Release,
                );
            }
        }

        tokio::select! {
            Some(req) = generate_image_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let stream = model.generate_image(&req.prompt, &req.config, &req.device);
                let _ = req.response.send(Ok(stream));

                // Transition: Processing → Ready
                state.store(WorkerState::Ready as u32, std::sync::atomic::Ordering::Release);
                last_activity = SystemTime::now();
            }
            Some(_ping) = health_rx.recv() => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let pong = HealthPong {
                    worker_id,
                    timestamp: now,
                    queue_depth: 0, // Note: tokio mpsc doesn't expose len()
                };

                let _ = health_tx.send(pong);
            }
            Some(_) = shutdown_rx.recv() => {
                log::info!("TextToImage worker {} shutting down", worker_id);
                // Transition: Ready/Idle → Evicting
                state.store(WorkerState::Evicting as u32, std::sync::atomic::Ordering::Release);
                break;
            }
        }
    }
}

/// Global TextToImage pool instance
static TEXT_TO_IMAGE_POOL: Lazy<Pool<TextToImageWorkerHandle>> =
    Lazy::new(|| Pool::new(PoolConfig::default()));

/// Access global TextToImage pool
pub fn text_to_image_pool() -> &'static Pool<TextToImageWorkerHandle> {
    &TEXT_TO_IMAGE_POOL
}

impl Pool<TextToImageWorkerHandle> {
    /// Spawn worker for TextToImage model
    pub fn spawn_text_to_image_worker<T, F, Fut>(
        &self,
        registry_key: &str,
        model_loader: F,
        per_worker_mb: usize,
        allocation_guard: AllocationGuard,
    ) -> Result<(), PoolError>
    where
        T: TextToImageCapable + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, PoolError>> + Send + 'static,
    {
        // Create unbounded channels for worker communication
        let (generate_image_tx, generate_image_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
        let (health_tx_main, health_rx_worker) = mpsc::unbounded_channel();
        let (health_tx_worker, health_rx_main) = mpsc::unbounded_channel();

        // Get worker ID before moving into thread
        let worker_id = self.next_worker_id();
        let registry_key_clone = registry_key.to_string();
        let per_worker_mb_clone = per_worker_mb;

        // Create state before spawning thread so we can clone it
        use std::sync::atomic::AtomicU32;
        let state = Arc::new(AtomicU32::new(0)); // Spawning state
        let state_clone = Arc::clone(&state);

        // Spawn worker task
        tokio::spawn(async move {
            use crate::capability::registry::pool::core::worker_state::WorkerState;

            // Guard held by worker task - will drop on exit
            let _memory_guard = allocation_guard;

            // Transition: Spawning → Loading
            state_clone.store(
                WorkerState::Loading as u32,
                std::sync::atomic::Ordering::Release,
            );

            let model = match model_loader().await {
                Ok(m) => {
                    log::info!("TextToImage worker {} ready", worker_id);
                    // Transition: Loading → Ready
                    state_clone.store(
                        WorkerState::Ready as u32,
                        std::sync::atomic::Ordering::Release,
                    );
                    m
                }
                Err(e) => {
                    log::error!(
                        "TextToImage worker {} model loading failed: {}",
                        worker_id,
                        e
                    );
                    // Transition: Loading → Failed
                    state_clone.store(
                        WorkerState::Failed as u32,
                        std::sync::atomic::Ordering::Release,
                    );

                    // Clean up memory tracking (CRITICAL FIX)
                    // This prevents memory leak when model loading fails
                    text_to_image_pool().remove_memory(per_worker_mb_clone);

                    return;
                }
            };

            text_to_image_worker(
                model,
                generate_image_rx,
                shutdown_rx,
                health_rx_worker,
                health_tx_worker,
                worker_id,
                Arc::clone(&state_clone),
            )
            .await;

            // Transition: Ready → Dead (when worker loop exits)
            state_clone.store(
                WorkerState::Dead as u32,
                std::sync::atomic::Ordering::Release,
            );
        });

        // Create handles (state already created above before spawning)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let pending_requests = Arc::new(AtomicUsize::new(0));
        let last_used = Arc::new(AtomicU64::new(now));

        // Store capability-specific handle
        let full_handle = TextToImageWorkerHandle {
            core: WorkerHandle {
                pending_requests,
                last_used,
                worker_id,
                shutdown_tx: shutdown_tx.clone(),
                per_worker_mb,
                health_tx: health_tx_main,
                health_rx: Arc::new(tokio::sync::Mutex::new(health_rx_main)),
                state,
            },
            generate_image_tx,
            shutdown_tx,
            registry_key: registry_key_clone.clone(),
        };

        // Single registration point - no duplication
        self.register_worker(registry_key.to_string(), full_handle);

        // Update memory tracking
        self.add_memory(per_worker_mb);

        Ok(())
    }

    /// Generate image using pooled worker (returns tokio Stream)
    pub fn generate_image(
        &self,
        registry_key: &str,
        prompt: &str,
        config: &ImageGenerationConfig,
        device: &Device,
    ) -> Pin<Box<dyn Stream<Item = ImageGenerationChunk> + Send>> {
        // Clone for move into closure
        let registry_key = registry_key.to_string();
        let prompt = prompt.to_string();
        let config = config.clone();
        let device = device.clone();
        let is_shutting_down = self.is_shutting_down();
        let request_timeout_secs = self.config().request_timeout_secs;

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Check shutdown
            if is_shutting_down {
                let _ = tx.send(ImageGenerationChunk::Error(
                    "Pool shutting down".to_string(),
                ));
                return;
            }

            // Get circuit breaker for this model and check state
            let pool = text_to_image_pool();
            let circuit = pool.get_circuit_breaker(&registry_key);

            if !circuit.can_request() {
                let _ = tx.send(ImageGenerationChunk::Error(format!(
                    "Circuit breaker open for {}",
                    registry_key
                )));
                // Update metrics
                pool.metrics()
                    .circuit_rejections
                    .fetch_add(1, Ordering::Relaxed);
                return;
            }

            // Get workers from pool
            let workers = match pool.workers().get(&registry_key) {
                Some(w) => w,
                None => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "No workers for {}",
                        registry_key
                    )));
                    return;
                }
            };

            if workers.is_empty() {
                let _ = tx.send(ImageGenerationChunk::Error(
                    "No workers available".to_string(),
                ));
                return;
            }

            // Find alive worker with least load using Power of Two Choices (O(1))
            let alive_workers: Vec<_> = workers.iter().filter(|w| w.is_alive()).collect();
            let worker = match select_worker_power_of_two(&alive_workers, |w| &w.core) {
                Some(w) => w,
                None => {
                    let _ = tx.send(ImageGenerationChunk::Error(format!(
                        "No alive workers for {}",
                        registry_key
                    )));
                    return;
                }
            };

            // Track request
            worker.core.pending_requests.fetch_add(1, Ordering::Relaxed);
            let _guard = PendingRequestsGuard::new(&worker.core.pending_requests);
            worker.core.touch();

            // Send request to worker
            let (response_tx, response_rx) = oneshot::channel();
            if let Err(e) = worker.generate_image_tx.send(GenerateImageRequest {
                prompt,
                config,
                device,
                response: response_tx,
            }) {
                let _ = tx.send(ImageGenerationChunk::Error(format!(
                    "Failed to send request: {}",
                    e
                )));
                return;
            }

            // Wait for worker's stream with timeout
            let timeout = Duration::from_secs(request_timeout_secs);
            let mut worker_stream = match tokio::time::timeout(timeout, response_rx).await {
                Ok(Ok(Ok(stream))) => {
                    // timeout Ok, recv Ok, result Ok
                    circuit.record_success();
                    stream
                }
                Ok(Ok(Err(e))) => {
                    // timeout Ok, recv Ok, result Err
                    circuit.record_failure();
                    pool.metrics().total_errors.fetch_add(1, Ordering::Relaxed);

                    let _ = tx.send(ImageGenerationChunk::Error(format!("Worker error: {}", e)));
                    return;
                }
                Ok(Err(_)) => {
                    // timeout Ok, recv Err (channel closed)
                    circuit.record_failure();
                    pool.metrics().total_errors.fetch_add(1, Ordering::Relaxed);

                    let _ = tx.send(ImageGenerationChunk::Error(
                        "Response channel closed".to_string(),
                    ));
                    return;
                }
                Err(_) => {
                    // timeout Err
                    circuit.record_failure();
                    pool.metrics()
                        .total_timeouts
                        .fetch_add(1, Ordering::Relaxed);

                    let _ = tx.send(ImageGenerationChunk::Error("Request timeout".to_string()));
                    return;
                }
            };

            // Forward chunks from worker stream to caller as they arrive
            use tokio_stream::StreamExt;
            while let Some(chunk) = worker_stream.next().await {
                if tx.send(chunk).is_err() {
                    break;
                }
            }
        }))
    }
}
