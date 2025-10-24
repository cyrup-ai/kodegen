use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Instant, sleep};

use crate::capability::registry::pool::WorkerState;
use crate::capability::registry::pool::core::memory_governor::AllocationGuard;
use crate::capability::registry::pool::core::types::{
    HealthPing, HealthPong, PendingRequestsGuard, select_worker_power_of_two,
};
use crate::capability::registry::pool::core::{Pool, PoolConfig, PoolError, WorkerHandle};
use crate::capability::traits::TextEmbeddingCapable;

/// Request for embed() operation
pub struct EmbedRequest {
    pub text: Arc<str>,
    pub task: Option<String>,
    pub response: oneshot::Sender<Result<Vec<f32>, PoolError>>,
}

/// Request for batch_embed() operation
pub struct BatchEmbedRequest {
    pub texts: Arc<[String]>,
    pub task: Option<String>,
    pub response: oneshot::Sender<Result<Vec<Vec<f32>>, PoolError>>,
}

/// TextEmbedding-specific worker handle with channels
#[derive(Clone)]
pub struct TextEmbeddingWorkerHandle {
    pub core: WorkerHandle,
    pub embed_tx: mpsc::Sender<EmbedRequest>,
    pub batch_embed_tx: mpsc::Sender<BatchEmbedRequest>,
    pub shutdown_tx: mpsc::UnboundedSender<()>,
    pub registry_key: String, // Added to enable cleanup on drop
}

impl crate::capability::registry::pool::core::types::PoolWorkerHandle
    for TextEmbeddingWorkerHandle
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

impl std::ops::Deref for TextEmbeddingWorkerHandle {
    type Target = WorkerHandle;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// Channels used by text embedding worker
pub struct TextEmbeddingWorkerChannels {
    pub embed_rx: mpsc::Receiver<EmbedRequest>,
    pub batch_embed_rx: mpsc::Receiver<BatchEmbedRequest>,
    pub shutdown_rx: mpsc::UnboundedReceiver<()>,
    pub health_rx: mpsc::UnboundedReceiver<HealthPing>,
    pub health_tx: mpsc::UnboundedSender<HealthPong>,
}

/// Context for text embedding worker
pub struct TextEmbeddingWorkerContext {
    pub worker_id: usize,
    pub registry_key: String,
    pub state: Arc<AtomicU32>,
}

/// Worker loop for TextEmbedding models
///
/// Processes requests from 2 channels:
/// - embed_rx: Single text embedding
/// - batch_embed_rx: Batch text embedding
///
/// Worker owns model exclusively, processes requests until shutdown.
pub async fn text_embedding_worker<T: TextEmbeddingCapable>(
    model: T,
    channels: TextEmbeddingWorkerChannels,
    context: TextEmbeddingWorkerContext,
) {
    use crate::capability::registry::pool::core::worker_state::WorkerState;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    // Destructure channels and context
    let TextEmbeddingWorkerChannels {
        mut embed_rx,
        mut batch_embed_rx,
        mut shutdown_rx,
        mut health_rx,
        health_tx,
    } = channels;
    let TextEmbeddingWorkerContext {
        worker_id,
        registry_key: _registry_key,
        state,
    } = context;

    // Setup idle timeout (Ready → Idle after 5 minutes of inactivity)
    let idle_threshold = Duration::from_secs(300);
    let timeout = sleep(idle_threshold);
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                let current_state = WorkerState::from(state.load(Ordering::Acquire));
                if matches!(current_state, WorkerState::Ready) {
                    state.store(WorkerState::Idle as u32, Ordering::Release);
                }
                timeout.as_mut().reset(Instant::now() + idle_threshold);
            }
            Some(req) = embed_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let result = model.embed(&req.text, req.task)
                    .await
                    .map_err(|e| PoolError::ModelError(e.to_string()));
                if let Err(e) = req.response.send(result) {
                    log::warn!(
                        "Worker {}: Failed to send response (client likely timed out): {:?}",
                        worker_id,
                        e
                    );
                }

                // Transition: Processing → Ready
                state.store(WorkerState::Ready as u32, std::sync::atomic::Ordering::Release);
                timeout.as_mut().reset(Instant::now() + idle_threshold);
            }
            Some(req) = batch_embed_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let result = model.batch_embed(&req.texts, req.task)
                    .await
                    .map_err(|e| PoolError::ModelError(e.to_string()));
                if let Err(e) = req.response.send(result) {
                    log::warn!(
                        "Worker {}: Failed to send response (client likely timed out): {:?}",
                        worker_id,
                        e
                    );
                }

                // Transition: Processing → Ready
                state.store(WorkerState::Ready as u32, std::sync::atomic::Ordering::Release);
                timeout.as_mut().reset(Instant::now() + idle_threshold);
            }
            Some(_ping) = health_rx.recv() => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let pong = HealthPong {
                    worker_id,
                    timestamp: now,
                    queue_depth: embed_rx.len() + batch_embed_rx.len(),
                };

                if let Err(e) = health_tx.send(pong) {
                    log::error!(
                        "Worker {}: Health channel broken: {:?}",
                        worker_id,
                        e
                    );
                }
            }
            Some(_) = shutdown_rx.recv() => {
                log::info!("TextEmbedding worker {} shutting down", worker_id);
                // Transition: Ready/Idle → Evicting
                state.store(WorkerState::Evicting as u32, std::sync::atomic::Ordering::Release);
                break;
            }
        }
    }
}

/// Global TextEmbedding pool instance  
static TEXT_EMBEDDING_POOL: Lazy<Pool<TextEmbeddingWorkerHandle>> =
    Lazy::new(|| Pool::new(PoolConfig::default()));

/// Access global TextEmbedding pool
pub fn text_embedding_pool() -> &'static Pool<TextEmbeddingWorkerHandle> {
    &TEXT_EMBEDDING_POOL
}

impl Pool<TextEmbeddingWorkerHandle> {
    /// Spawn worker for TextEmbedding model
    pub fn spawn_text_embedding_worker<T, F, Fut>(
        &self,
        registry_key: &str,
        model_loader: F,
        per_worker_mb: usize,
        allocation_guard: AllocationGuard,
    ) -> Result<(), PoolError>
    where
        T: TextEmbeddingCapable + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, PoolError>> + Send + 'static,
    {
        // Access config for channel capacities
        let config = self.config();

        // Create bounded channels with configured capacities
        let (embed_tx, embed_rx) = mpsc::channel(config.embed_queue_capacity);
        let (batch_embed_tx, batch_embed_rx) = mpsc::channel(config.batch_queue_capacity);

        // Shutdown stays unbounded (only 1 message ever sent)
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();

        // Health channels stay unbounded (WorkerHandle in core requires unbounded)
        let (health_tx_main, health_rx_worker) = mpsc::unbounded_channel();
        let (health_tx_worker, health_rx_main) = mpsc::unbounded_channel();

        // Get worker ID before moving into task
        let worker_id = self.next_worker_id();
        let registry_key_str = registry_key.to_string();

        // Create state for worker
        use std::sync::Arc;
        // Clone channels for worker task
        let health_tx_worker_clone = health_tx_worker.clone();
        let per_worker_mb_clone = per_worker_mb;

        // Create state before spawning thread so we can clone it
        use std::sync::atomic::AtomicU32;
        let state = Arc::new(AtomicU32::new(0)); // Spawning state
        let state_for_task = Arc::clone(&state);

        // Clone channels for registration inside task after model loads
        let embed_tx_clone = embed_tx.clone();
        let batch_embed_tx_clone = batch_embed_tx.clone();
        let shutdown_tx_clone = shutdown_tx.clone();
        let health_tx_main_clone = health_tx_main.clone();

        // Spawn worker task
        tokio::spawn(async move {
            // Guard held by worker task - will drop on exit
            let _memory_guard = allocation_guard;

            // Transition: Spawning → Loading
            state_for_task.store(
                WorkerState::Loading as u32,
                std::sync::atomic::Ordering::Release,
            );

            // Load model
            let model = match model_loader().await {
                Ok(m) => {
                    log::info!("TextEmbedding worker {} ready", worker_id);
                    // Transition: Loading → Ready
                    state_for_task.store(
                        WorkerState::Ready as u32,
                        std::sync::atomic::Ordering::Release,
                    );
                    m
                }
                Err(e) => {
                    log::error!("TextEmbedding worker {} failed: {}", worker_id, e);
                    // Transition: Loading → Failed
                    state_for_task.store(
                        WorkerState::Failed as u32,
                        std::sync::atomic::Ordering::Release,
                    );

                    // No cleanup needed - worker was never registered
                    // AllocationGuard will auto-release memory on return
                    return; // Exit thread without running worker loop
                }
            };

            // Model loaded successfully - NOW create and register worker
            use std::sync::atomic::{AtomicU64, AtomicUsize};
            use std::time::{SystemTime, UNIX_EPOCH};

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let pending_requests = Arc::new(AtomicUsize::new(0));
            let last_used = Arc::new(AtomicU64::new(now));

            // Clone registry_key for handle and worker context, use original for registration
            let registry_key_for_worker = registry_key_str.clone();
            let registry_key_for_handle = registry_key_str.clone();

            let full_handle = TextEmbeddingWorkerHandle {
                core: WorkerHandle {
                    pending_requests,
                    last_used,
                    worker_id,
                    shutdown_tx: shutdown_tx_clone.clone(),
                    per_worker_mb: per_worker_mb_clone,
                    health_tx: health_tx_main_clone.clone(),
                    health_rx: Arc::new(tokio::sync::Mutex::new(health_rx_main)),
                    state: Arc::clone(&state_for_task),
                },
                embed_tx: embed_tx_clone,
                batch_embed_tx: batch_embed_tx_clone,
                shutdown_tx: shutdown_tx_clone,
                registry_key: registry_key_for_handle,
            };

            text_embedding_pool().register_worker(registry_key_str, full_handle);
            text_embedding_pool().add_memory(per_worker_mb_clone);

            text_embedding_worker(
                model,
                TextEmbeddingWorkerChannels {
                    embed_rx,
                    batch_embed_rx,
                    shutdown_rx,
                    health_rx: health_rx_worker,
                    health_tx: health_tx_worker_clone,
                },
                TextEmbeddingWorkerContext {
                    worker_id,
                    registry_key: registry_key_for_worker,
                    state: Arc::clone(&state_for_task),
                },
            )
            .await;

            // Transition: Ready → Dead (when worker loop exits)
            state_for_task.store(
                WorkerState::Dead as u32,
                std::sync::atomic::Ordering::Release,
            );
        });

        Ok(())
    }

    /// Embed text using pooled worker
    pub async fn embed_text(
        &self,
        registry_key: &str,
        text: &str,
        task: Option<String>,
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
        let alive_workers: Vec<_> = workers.iter().filter(|w| w.core.is_alive()).collect();

        let worker = select_worker_power_of_two(&alive_workers, |w| &w.core).ok_or_else(|| {
            PoolError::NoWorkers(format!("No alive workers for {}", registry_key))
        })?;

        // Send request with automatic counter cleanup
        worker.core.pending_requests.fetch_add(1, Ordering::Relaxed);
        let _guard = PendingRequestsGuard::new(&worker.core.pending_requests);
        worker.core.touch();

        let (response_tx, response_rx) = oneshot::channel();
        worker
            .embed_tx
            .send(EmbedRequest {
                text: Arc::from(text),
                task,
                response: response_tx,
            })
            .await
            .map_err(|e| PoolError::SendError(e.to_string()))?;

        // Wait for response with timeout
        let timeout = Duration::from_secs(self.config().request_timeout_secs);
        let result = match tokio::time::timeout(timeout, response_rx).await {
            Err(_) => {
                circuit.record_failure();
                self.metrics()
                    .total_timeouts
                    .fetch_add(1, Ordering::Relaxed);
                Err(PoolError::Timeout("Request timed out".to_string()))
            }
            Ok(Err(_)) => {
                circuit.record_failure();
                self.metrics().total_errors.fetch_add(1, Ordering::Relaxed);
                Err(PoolError::RecvError("Response channel closed".to_string()))
            }
            Ok(Ok(res)) => res,
        };

        // Record success or failure based on result
        match &result {
            Ok(_) => circuit.record_success(),
            Err(_) => {
                // Already recorded above in unified error handling
            }
        }

        result
    }

    /// Batch embed texts using pooled worker
    pub async fn batch_embed_text(
        &self,
        registry_key: &str,
        texts: &[String],
        task: Option<String>,
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
        let alive_workers: Vec<_> = workers.iter().filter(|w| w.core.is_alive()).collect();

        let worker = select_worker_power_of_two(&alive_workers, |w| &w.core).ok_or_else(|| {
            PoolError::NoWorkers(format!("No alive workers for {}", registry_key))
        })?;

        // Send request with automatic counter cleanup
        worker.core.pending_requests.fetch_add(1, Ordering::Relaxed);
        let _guard = PendingRequestsGuard::new(&worker.core.pending_requests);
        worker.core.touch();

        let (response_tx, response_rx) = oneshot::channel();
        worker
            .batch_embed_tx
            .send(BatchEmbedRequest {
                texts: Arc::from(texts),
                task,
                response: response_tx,
            })
            .await
            .map_err(|e| PoolError::SendError(e.to_string()))?;

        // Wait for response with timeout
        let timeout = Duration::from_secs(self.config().request_timeout_secs);
        let result = match tokio::time::timeout(timeout, response_rx).await {
            Err(_) => {
                circuit.record_failure();
                self.metrics()
                    .total_timeouts
                    .fetch_add(1, Ordering::Relaxed);
                Err(PoolError::Timeout("Request timed out".to_string()))
            }
            Ok(Err(_)) => {
                circuit.record_failure();
                self.metrics().total_errors.fetch_add(1, Ordering::Relaxed);
                Err(PoolError::RecvError("Response channel closed".to_string()))
            }
            Ok(Ok(res)) => res,
        };

        // Record success or failure based on result
        match &result {
            Ok(_) => circuit.record_success(),
            Err(_) => {
                // Already recorded above in unified error handling
            }
        }

        result
    }
}
