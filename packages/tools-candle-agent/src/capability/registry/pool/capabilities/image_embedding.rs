use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};

use crate::capability::registry::pool::core::memory_governor::AllocationGuard;
use crate::capability::registry::pool::core::types::{
    HealthPing, HealthPong, PendingRequestsGuard, select_worker_power_of_two,
};
use crate::capability::registry::pool::core::{Pool, PoolConfig, PoolError, WorkerHandle};
use crate::capability::traits::ImageEmbeddingCapable;

/// Request for embed_image() operation
pub struct EmbedImageRequest {
    pub image_path: String,
    pub response: oneshot::Sender<Result<Vec<f32>, PoolError>>,
}

/// Request for embed_image_url() operation
pub struct EmbedImageUrlRequest {
    pub url: String,
    pub response: oneshot::Sender<Result<Vec<f32>, PoolError>>,
}

/// Request for embed_image_base64() operation
pub struct EmbedImageBase64Request {
    pub base64_data: String,
    pub response: oneshot::Sender<Result<Vec<f32>, PoolError>>,
}

/// Request for batch_embed_images() operation
pub struct BatchEmbedImagesRequest {
    pub image_paths: Vec<String>,
    pub response: oneshot::Sender<Result<Vec<Vec<f32>>, PoolError>>,
}

/// ImageEmbedding-specific worker handle with channels
#[derive(Clone)]
pub struct ImageEmbeddingWorkerHandle {
    pub core: WorkerHandle,
    pub embed_image_tx: mpsc::UnboundedSender<EmbedImageRequest>,
    pub embed_image_url_tx: mpsc::UnboundedSender<EmbedImageUrlRequest>,
    pub embed_image_base64_tx: mpsc::UnboundedSender<EmbedImageBase64Request>,
    pub batch_embed_images_tx: mpsc::UnboundedSender<BatchEmbedImagesRequest>,
    pub shutdown_tx: mpsc::UnboundedSender<()>,
    pub registry_key: String, // Added to enable cleanup on drop
}

impl crate::capability::registry::pool::core::types::PoolWorkerHandle
    for ImageEmbeddingWorkerHandle
{
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

impl std::ops::Deref for ImageEmbeddingWorkerHandle {
    type Target = WorkerHandle;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// Channels used by image embedding worker
pub struct ImageEmbeddingWorkerChannels {
    pub embed_image_rx: mpsc::UnboundedReceiver<EmbedImageRequest>,
    pub embed_image_url_rx: mpsc::UnboundedReceiver<EmbedImageUrlRequest>,
    pub embed_image_base64_rx: mpsc::UnboundedReceiver<EmbedImageBase64Request>,
    pub batch_embed_images_rx: mpsc::UnboundedReceiver<BatchEmbedImagesRequest>,
    pub shutdown_rx: mpsc::UnboundedReceiver<()>,
    pub health_rx: mpsc::UnboundedReceiver<HealthPing>,
    pub health_tx: mpsc::UnboundedSender<HealthPong>,
}

/// Context for image embedding worker
pub struct ImageEmbeddingWorkerContext {
    pub worker_id: usize,
    pub state: Arc<AtomicU32>,
}

/// Worker loop for ImageEmbedding models
///
/// Processes requests from 4 channels:
/// - embed_image_rx: Single image embedding from path
/// - embed_image_url_rx: Single image embedding from URL
/// - embed_image_base64_rx: Single image embedding from base64
/// - batch_embed_images_rx: Batch image embedding
///
/// Worker owns model exclusively, processes requests until shutdown.
pub async fn image_embedding_worker<T: ImageEmbeddingCapable>(
    model: T,
    channels: ImageEmbeddingWorkerChannels,
    context: ImageEmbeddingWorkerContext,
) {
    use crate::capability::registry::pool::core::worker_state::WorkerState;
    use std::time::{Duration, SystemTime};

    // Destructure channels and context
    let ImageEmbeddingWorkerChannels {
        mut embed_image_rx,
        mut embed_image_url_rx,
        mut embed_image_base64_rx,
        mut batch_embed_images_rx,
        mut shutdown_rx,
        mut health_rx,
        health_tx,
    } = channels;
    let ImageEmbeddingWorkerContext { worker_id, state } = context;

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
            Some(req) = embed_image_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let result = model.embed_image(&req.image_path)
                    .await
                    .map_err(|e| PoolError::ModelError(e.to_string()));
                let _ = req.response.send(result);

                // Transition: Processing → Ready
                state.store(WorkerState::Ready as u32, std::sync::atomic::Ordering::Release);
                last_activity = SystemTime::now();
            }
            Some(req) = embed_image_url_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let result = model.embed_image_url(&req.url)
                    .await
                    .map_err(|e| PoolError::ModelError(e.to_string()));
                let _ = req.response.send(result);

                // Transition: Processing → Ready
                state.store(WorkerState::Ready as u32, std::sync::atomic::Ordering::Release);
                last_activity = SystemTime::now();
            }
            Some(req) = embed_image_base64_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let result = model.embed_image_base64(&req.base64_data)
                    .await
                    .map_err(|e| PoolError::ModelError(e.to_string()));
                let _ = req.response.send(result);

                // Transition: Processing → Ready
                state.store(WorkerState::Ready as u32, std::sync::atomic::Ordering::Release);
                last_activity = SystemTime::now();
            }
            Some(req) = batch_embed_images_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let paths: Vec<&str> = req.image_paths.iter().map(|s| s.as_str()).collect();
                let result = model.batch_embed_images(paths)
                    .await
                    .map_err(|e| PoolError::ModelError(e.to_string()));
                let _ = req.response.send(result);

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
                log::info!("ImageEmbedding worker {} shutting down", worker_id);
                // Transition: Ready/Idle → Evicting
                state.store(WorkerState::Evicting as u32, std::sync::atomic::Ordering::Release);
                break;
            }
        }
    }
}

/// Global ImageEmbedding pool instance
static IMAGE_EMBEDDING_POOL: Lazy<Pool<ImageEmbeddingWorkerHandle>> =
    Lazy::new(|| Pool::new(PoolConfig::default()));

/// Access global ImageEmbedding pool
pub fn image_embedding_pool() -> &'static Pool<ImageEmbeddingWorkerHandle> {
    &IMAGE_EMBEDDING_POOL
}

impl Pool<ImageEmbeddingWorkerHandle> {
    /// Spawn worker for ImageEmbedding model
    pub fn spawn_image_embedding_worker<T, F, Fut>(
        &self,
        registry_key: &str,
        model_loader: F,
        per_worker_mb: usize,
        allocation_guard: AllocationGuard,
    ) -> Result<(), PoolError>
    where
        T: ImageEmbeddingCapable + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, PoolError>> + Send + 'static,
    {
        // Create unbounded channels for worker communication
        let (embed_image_tx, embed_image_rx) = mpsc::unbounded_channel();
        let (embed_image_url_tx, embed_image_url_rx) = mpsc::unbounded_channel();
        let (embed_image_base64_tx, embed_image_base64_rx) = mpsc::unbounded_channel();
        let (batch_embed_images_tx, batch_embed_images_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
        let (health_tx_main, health_rx_worker) = mpsc::unbounded_channel();
        let (health_tx_worker, health_rx_main) = mpsc::unbounded_channel();

        // Get worker ID before moving into task
        let worker_id = self.next_worker_id();
        let registry_key_clone = registry_key.to_string();

        // Clone channels for worker task
        let health_tx_worker_clone = health_tx_worker.clone();
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
                    log::info!("ImageEmbedding worker {} ready", worker_id);
                    // Transition: Loading → Ready
                    state_clone.store(
                        WorkerState::Ready as u32,
                        std::sync::atomic::Ordering::Release,
                    );
                    m
                }
                Err(e) => {
                    log::error!(
                        "ImageEmbedding worker {} model loading failed: {}",
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
                    image_embedding_pool().remove_memory(per_worker_mb_clone);

                    return;
                }
            };

            image_embedding_worker(
                model,
                ImageEmbeddingWorkerChannels {
                    embed_image_rx,
                    embed_image_url_rx,
                    embed_image_base64_rx,
                    batch_embed_images_rx,
                    shutdown_rx,
                    health_rx: health_rx_worker,
                    health_tx: health_tx_worker_clone,
                },
                ImageEmbeddingWorkerContext {
                    worker_id,
                    state: Arc::clone(&state_clone),
                },
            )
            .await;

            // Transition: Ready → Dead (when worker loop exits)
            state_clone.store(
                WorkerState::Dead as u32,
                std::sync::atomic::Ordering::Release,
            );
        });

        // Create handles
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let pending_requests = Arc::new(AtomicUsize::new(0));
        let last_used = Arc::new(AtomicU64::new(now));

        // Store capability-specific handle (state already created above before spawning)
        let full_handle = ImageEmbeddingWorkerHandle {
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
            embed_image_tx,
            embed_image_url_tx,
            embed_image_base64_tx,
            batch_embed_images_tx,
            shutdown_tx,
            registry_key: registry_key_clone.clone(),
        };

        // Single registration point - no duplication
        self.register_worker(registry_key.to_string(), full_handle);

        // Update memory tracking
        self.add_memory(per_worker_mb);

        Ok(())
    }

    /// Embed image using pooled worker
    pub async fn embed_image(
        &self,
        registry_key: &str,
        image_path: &str,
    ) -> Result<Vec<f32>, PoolError> {
        // Check shutdown
        if self.is_shutting_down() {
            return Err(PoolError::ShuttingDown("Pool shutting down".to_string()));
        }

        // Get circuit breaker for this model and check state
        let circuit = self.get_circuit_breaker(registry_key);

        if !circuit.can_request() {
            self.metrics()
                .circuit_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(PoolError::CircuitOpen(format!(
                "Circuit breaker open for {}",
                registry_key
            )));
        }

        // Get workers from pool
        let workers = self
            .workers()
            .get(registry_key)
            .ok_or_else(|| PoolError::NoWorkers(format!("No workers for {}", registry_key)))?;

        if workers.is_empty() {
            return Err(PoolError::NoWorkers("No workers available".to_string()));
        }

        // Find alive worker with least load using Power of Two Choices (O(1))
        let alive_workers: Vec<_> = workers.iter().filter(|w| w.is_alive()).collect();
        let worker = select_worker_power_of_two(&alive_workers, |w| &w.core).ok_or_else(|| {
            PoolError::NoWorkers(format!("No alive workers for {}", registry_key))
        })?;

        // Send request
        worker.core.pending_requests.fetch_add(1, Ordering::Relaxed);
        let _guard = PendingRequestsGuard::new(&worker.core.pending_requests);
        worker.core.touch();

        let (response_tx, response_rx) = oneshot::channel();
        worker
            .embed_image_tx
            .send(EmbedImageRequest {
                image_path: image_path.to_string(),
                response: response_tx,
            })
            .map_err(|e| PoolError::SendError(e.to_string()))?;

        // Wait for response with timeout
        let timeout = Duration::from_secs(self.config().request_timeout_secs);
        let result = tokio::time::timeout(timeout, response_rx)
            .await
            .map_err(|_| {
                // Record timeout as failure
                circuit.record_failure();
                self.metrics()
                    .total_timeouts
                    .fetch_add(1, Ordering::Relaxed);
                PoolError::Timeout("Request timed out".to_string())
            })?
            .map_err(|_| PoolError::RecvError("Response channel closed".to_string()))?;

        // Record success or failure based on result
        match &result {
            Ok(_) => circuit.record_success(),
            Err(_) => {
                circuit.record_failure();
                self.metrics().total_errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        result
    }

    /// Embed image from URL using pooled worker
    pub async fn embed_image_url(
        &self,
        registry_key: &str,
        url: &str,
    ) -> Result<Vec<f32>, PoolError> {
        // Check shutdown
        if self.is_shutting_down() {
            return Err(PoolError::ShuttingDown("Pool shutting down".to_string()));
        }

        // Get circuit breaker for this model and check state
        let circuit = self.get_circuit_breaker(registry_key);

        if !circuit.can_request() {
            self.metrics()
                .circuit_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(PoolError::CircuitOpen(format!(
                "Circuit breaker open for {}",
                registry_key
            )));
        }

        // Get workers from pool
        let workers = self
            .workers()
            .get(registry_key)
            .ok_or_else(|| PoolError::NoWorkers(format!("No workers for {}", registry_key)))?;

        if workers.is_empty() {
            return Err(PoolError::NoWorkers("No workers available".to_string()));
        }

        // Find alive worker with least load using Power of Two Choices (O(1))
        let alive_workers: Vec<_> = workers.iter().filter(|w| w.is_alive()).collect();
        let worker = select_worker_power_of_two(&alive_workers, |w| &w.core).ok_or_else(|| {
            PoolError::NoWorkers(format!("No alive workers for {}", registry_key))
        })?;

        // Send request
        worker.core.pending_requests.fetch_add(1, Ordering::Relaxed);
        let _guard = PendingRequestsGuard::new(&worker.core.pending_requests);
        worker.core.touch();

        let (response_tx, response_rx) = oneshot::channel();
        worker
            .embed_image_url_tx
            .send(EmbedImageUrlRequest {
                url: url.to_string(),
                response: response_tx,
            })
            .map_err(|e| PoolError::SendError(e.to_string()))?;

        // Wait for response with timeout
        let timeout = Duration::from_secs(self.config().request_timeout_secs);
        let result = tokio::time::timeout(timeout, response_rx)
            .await
            .map_err(|_| {
                // Record timeout as failure
                circuit.record_failure();
                self.metrics()
                    .total_timeouts
                    .fetch_add(1, Ordering::Relaxed);
                PoolError::Timeout("Request timed out".to_string())
            })?
            .map_err(|_| PoolError::RecvError("Response channel closed".to_string()))?;

        // Record success or failure based on result
        match &result {
            Ok(_) => circuit.record_success(),
            Err(_) => {
                circuit.record_failure();
                self.metrics().total_errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        result
    }

    /// Embed image from base64 data using pooled worker
    pub async fn embed_image_base64(
        &self,
        registry_key: &str,
        base64_data: &str,
    ) -> Result<Vec<f32>, PoolError> {
        // Check shutdown
        if self.is_shutting_down() {
            return Err(PoolError::ShuttingDown("Pool shutting down".to_string()));
        }

        // Get circuit breaker for this model and check state
        let circuit = self.get_circuit_breaker(registry_key);

        if !circuit.can_request() {
            self.metrics()
                .circuit_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(PoolError::CircuitOpen(format!(
                "Circuit breaker open for {}",
                registry_key
            )));
        }

        // Get workers from pool
        let workers = self
            .workers()
            .get(registry_key)
            .ok_or_else(|| PoolError::NoWorkers(format!("No workers for {}", registry_key)))?;

        if workers.is_empty() {
            return Err(PoolError::NoWorkers("No workers available".to_string()));
        }

        // Find alive worker with least load using Power of Two Choices (O(1))
        let alive_workers: Vec<_> = workers.iter().filter(|w| w.is_alive()).collect();
        let worker = select_worker_power_of_two(&alive_workers, |w| &w.core).ok_or_else(|| {
            PoolError::NoWorkers(format!("No alive workers for {}", registry_key))
        })?;

        // Send request
        worker.core.pending_requests.fetch_add(1, Ordering::Relaxed);
        let _guard = PendingRequestsGuard::new(&worker.core.pending_requests);
        worker.core.touch();

        let (response_tx, response_rx) = oneshot::channel();
        worker
            .embed_image_base64_tx
            .send(EmbedImageBase64Request {
                base64_data: base64_data.to_string(),
                response: response_tx,
            })
            .map_err(|e| PoolError::SendError(e.to_string()))?;

        // Wait for response with timeout
        let timeout = Duration::from_secs(self.config().request_timeout_secs);
        let result = tokio::time::timeout(timeout, response_rx)
            .await
            .map_err(|_| {
                // Record timeout as failure
                circuit.record_failure();
                self.metrics()
                    .total_timeouts
                    .fetch_add(1, Ordering::Relaxed);
                PoolError::Timeout("Request timed out".to_string())
            })?
            .map_err(|_| PoolError::RecvError("Response channel closed".to_string()))?;

        // Record success or failure based on result
        match &result {
            Ok(_) => circuit.record_success(),
            Err(_) => {
                circuit.record_failure();
                self.metrics().total_errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        result
    }

    /// Batch embed images using pooled worker
    pub async fn batch_embed_images(
        &self,
        registry_key: &str,
        image_paths: &[String],
    ) -> Result<Vec<Vec<f32>>, PoolError> {
        // Check shutdown
        if self.is_shutting_down() {
            return Err(PoolError::ShuttingDown("Pool shutting down".to_string()));
        }

        // Get circuit breaker for this model and check state
        let circuit = self.get_circuit_breaker(registry_key);

        if !circuit.can_request() {
            self.metrics()
                .circuit_rejections
                .fetch_add(1, Ordering::Relaxed);
            return Err(PoolError::CircuitOpen(format!(
                "Circuit breaker open for {}",
                registry_key
            )));
        }

        // Get workers from pool
        let workers = self
            .workers()
            .get(registry_key)
            .ok_or_else(|| PoolError::NoWorkers(format!("No workers for {}", registry_key)))?;

        if workers.is_empty() {
            return Err(PoolError::NoWorkers("No workers available".to_string()));
        }

        // Find alive worker with least load using Power of Two Choices (O(1))
        let alive_workers: Vec<_> = workers.iter().filter(|w| w.is_alive()).collect();
        let worker = select_worker_power_of_two(&alive_workers, |w| &w.core).ok_or_else(|| {
            PoolError::NoWorkers(format!("No alive workers for {}", registry_key))
        })?;

        // Send request
        worker.core.pending_requests.fetch_add(1, Ordering::Relaxed);
        let _guard = PendingRequestsGuard::new(&worker.core.pending_requests);
        worker.core.touch();

        let (response_tx, response_rx) = oneshot::channel();
        worker
            .batch_embed_images_tx
            .send(BatchEmbedImagesRequest {
                image_paths: image_paths.to_vec(),
                response: response_tx,
            })
            .map_err(|e| PoolError::SendError(e.to_string()))?;

        // Wait for response with timeout
        let timeout = Duration::from_secs(self.config().request_timeout_secs);
        let result = tokio::time::timeout(timeout, response_rx)
            .await
            .map_err(|_| {
                // Record timeout as failure
                circuit.record_failure();
                self.metrics()
                    .total_timeouts
                    .fetch_add(1, Ordering::Relaxed);
                PoolError::Timeout("Request timed out".to_string())
            })?
            .map_err(|_| PoolError::RecvError("Response channel closed".to_string()))?;

        // Record success or failure based on result
        match &result {
            Ok(_) => circuit.record_success(),
            Err(_) => {
                circuit.record_failure();
                self.metrics().total_errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        result
    }
}
