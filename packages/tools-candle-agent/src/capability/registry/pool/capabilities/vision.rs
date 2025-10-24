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
use crate::capability::traits::VisionCapable;
use crate::domain::context::CandleStringChunk;

/// Type alias for vision streaming response sender
type VisionResponse =
    oneshot::Sender<Result<Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>>, PoolError>>;

/// Request for describe_image() operation (streaming response)
pub struct DescribeImageRequest {
    pub image_path: String,
    pub query: String,
    pub response: VisionResponse,
}

/// Request for describe_url() operation (streaming response)
pub struct DescribeUrlRequest {
    pub url: String,
    pub query: String,
    pub response: VisionResponse,
}

/// Vision-specific worker handle with channels
pub struct VisionWorkerHandle {
    pub core: WorkerHandle,
    pub describe_image_tx: mpsc::UnboundedSender<DescribeImageRequest>,
    pub describe_url_tx: mpsc::UnboundedSender<DescribeUrlRequest>,
    pub shutdown_tx: mpsc::UnboundedSender<()>,
    pub registry_key: String, // Added to enable cleanup on drop
}

impl crate::capability::registry::pool::core::types::PoolWorkerHandle for VisionWorkerHandle {
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

impl std::ops::Deref for VisionWorkerHandle {
    type Target = WorkerHandle;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// Channels used by vision worker
pub struct VisionWorkerChannels {
    pub describe_image_rx: mpsc::UnboundedReceiver<DescribeImageRequest>,
    pub describe_url_rx: mpsc::UnboundedReceiver<DescribeUrlRequest>,
    pub shutdown_rx: mpsc::UnboundedReceiver<()>,
    pub health_rx: mpsc::UnboundedReceiver<HealthPing>,
    pub health_tx: mpsc::UnboundedSender<HealthPong>,
}

/// Context for vision worker
pub struct VisionWorkerContext {
    pub worker_id: usize,
    pub state: Arc<AtomicU32>,
}

/// Worker loop for Vision models
///
/// Processes streaming requests. Worker calls trait method which
/// returns Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>>. Stream is sent back to caller
/// who forwards chunks to end user.
pub async fn vision_worker<T: VisionCapable>(
    model: T,
    channels: VisionWorkerChannels,
    context: VisionWorkerContext,
) {
    use crate::capability::registry::pool::core::worker_state::WorkerState;
    use std::time::{Duration, SystemTime};

    // Destructure channels and context
    let VisionWorkerChannels {
        mut describe_image_rx,
        mut describe_url_rx,
        mut shutdown_rx,
        mut health_rx,
        health_tx,
    } = channels;
    let VisionWorkerContext { worker_id, state } = context;

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
            Some(req) = describe_image_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let stream = model.describe_image(&req.image_path, &req.query);
                let _ = req.response.send(Ok(stream));

                // Transition: Processing → Ready
                state.store(WorkerState::Ready as u32, std::sync::atomic::Ordering::Release);
                last_activity = SystemTime::now();
            }
            Some(req) = describe_url_rx.recv() => {
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                let stream = model.describe_url(&req.url, &req.query);
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
                log::info!("Vision worker {} shutting down", worker_id);
                // Transition: Ready/Idle → Evicting
                state.store(WorkerState::Evicting as u32, std::sync::atomic::Ordering::Release);
                break;
            }
        }
    }
}

/// Global Vision pool instance
static VISION_POOL: Lazy<Pool<VisionWorkerHandle>> = Lazy::new(|| Pool::new(PoolConfig::default()));

/// Access global Vision pool
pub fn vision_pool() -> &'static Pool<VisionWorkerHandle> {
    &VISION_POOL
}

impl Pool<VisionWorkerHandle> {
    /// Spawn worker for Vision model
    pub fn spawn_vision_worker<T, F, Fut>(
        &self,
        registry_key: &str,
        model_loader: F,
        per_worker_mb: usize,
        allocation_guard: AllocationGuard,
    ) -> Result<(), PoolError>
    where
        T: VisionCapable + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, PoolError>> + Send + 'static,
    {
        // Create unbounded channels for worker communication
        let (describe_image_tx, describe_image_rx) = mpsc::unbounded_channel();
        let (describe_url_tx, describe_url_rx) = mpsc::unbounded_channel();
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
                    log::info!("Vision worker {} ready", worker_id);
                    // Transition: Loading → Ready
                    state_clone.store(
                        WorkerState::Ready as u32,
                        std::sync::atomic::Ordering::Release,
                    );
                    m
                }
                Err(e) => {
                    log::error!("Vision worker {} model loading failed: {}", worker_id, e);
                    // Transition: Loading → Failed
                    state_clone.store(
                        WorkerState::Failed as u32,
                        std::sync::atomic::Ordering::Release,
                    );

                    // Clean up memory tracking (CRITICAL FIX)
                    // This prevents memory leak when model loading fails
                    vision_pool().remove_memory(per_worker_mb_clone);

                    return;
                }
            };

            vision_worker(
                model,
                VisionWorkerChannels {
                    describe_image_rx,
                    describe_url_rx,
                    shutdown_rx,
                    health_rx: health_rx_worker,
                    health_tx: health_tx_worker,
                },
                VisionWorkerContext {
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

        // Create handles (state already created above before spawning)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let pending_requests = Arc::new(AtomicUsize::new(0));
        let last_used = Arc::new(AtomicU64::new(now));

        // Store capability-specific handle
        let full_handle = VisionWorkerHandle {
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
            describe_image_tx,
            describe_url_tx,
            shutdown_tx,
            registry_key: registry_key_clone.clone(),
        };

        // Single registration point - no duplication
        self.register_worker(registry_key.to_string(), full_handle);

        // Update memory tracking
        self.add_memory(per_worker_mb);

        Ok(())
    }

    /// Describe image using pooled worker (returns tokio Stream)
    pub fn describe_image(
        &self,
        registry_key: &str,
        image_path: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        // Clone for move into closure
        let registry_key = registry_key.to_string();
        let image_path = image_path.to_string();
        let query = query.to_string();
        let is_shutting_down = self.is_shutting_down();
        let request_timeout_secs = self.config().request_timeout_secs;

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Check shutdown
            if is_shutting_down {
                let _ = tx.send(CandleStringChunk::text("Pool shutting down".to_string()));
                return;
            }

            // Get circuit breaker for this model and check state
            let pool = vision_pool();
            let circuit = pool.get_circuit_breaker(&registry_key);

            if !circuit.can_request() {
                let _ = tx.send(CandleStringChunk::text(format!(
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
                    let _ = tx.send(CandleStringChunk::text(format!(
                        "No workers for {}",
                        registry_key
                    )));
                    return;
                }
            };

            if workers.is_empty() {
                let _ = tx.send(CandleStringChunk::text("No workers available".to_string()));
                return;
            }

            // Find alive worker with least load using Power of Two Choices (O(1))
            let alive_workers: Vec<_> = workers.iter().filter(|w| w.core.is_alive()).collect();

            let worker = match select_worker_power_of_two(&alive_workers, |w| &w.core) {
                Some(w) => w,
                None => {
                    let _ = tx.send(CandleStringChunk::text(format!(
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
            if let Err(e) = worker.describe_image_tx.send(DescribeImageRequest {
                image_path,
                query,
                response: response_tx,
            }) {
                let _ = tx.send(CandleStringChunk::text(format!(
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

                    let _ = tx.send(CandleStringChunk::text(format!("Worker error: {}", e)));
                    return;
                }
                Ok(Err(_)) => {
                    // timeout Ok, recv Err (channel closed)
                    circuit.record_failure();
                    pool.metrics().total_errors.fetch_add(1, Ordering::Relaxed);

                    let _ = tx.send(CandleStringChunk::text(
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

                    let _ = tx.send(CandleStringChunk::text("Request timeout".to_string()));
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

    /// Describe image from URL using pooled worker (returns tokio Stream)
    pub fn describe_url(
        &self,
        registry_key: &str,
        url: &str,
        query: &str,
    ) -> Pin<Box<dyn Stream<Item = CandleStringChunk> + Send>> {
        // Clone for move into closure
        let registry_key = registry_key.to_string();
        let url = url.to_string();
        let query = query.to_string();
        let is_shutting_down = self.is_shutting_down();
        let request_timeout_secs = self.config().request_timeout_secs;

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Check shutdown
            if is_shutting_down {
                let _ = tx.send(CandleStringChunk::text("Pool shutting down".to_string()));
                return;
            }

            // Get circuit breaker for this model and check state
            let pool = vision_pool();
            let circuit = pool.get_circuit_breaker(&registry_key);

            if !circuit.can_request() {
                let _ = tx.send(CandleStringChunk::text(format!(
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
                    let _ = tx.send(CandleStringChunk::text(format!(
                        "No workers for {}",
                        registry_key
                    )));
                    return;
                }
            };

            if workers.is_empty() {
                let _ = tx.send(CandleStringChunk::text("No workers available".to_string()));
                return;
            }

            // Find alive worker with least load using Power of Two Choices (O(1))
            let alive_workers: Vec<_> = workers.iter().filter(|w| w.core.is_alive()).collect();

            let worker = match select_worker_power_of_two(&alive_workers, |w| &w.core) {
                Some(w) => w,
                None => {
                    let _ = tx.send(CandleStringChunk::text(format!(
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
            if let Err(e) = worker.describe_url_tx.send(DescribeUrlRequest {
                url,
                query,
                response: response_tx,
            }) {
                let _ = tx.send(CandleStringChunk::text(format!(
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

                    let _ = tx.send(CandleStringChunk::text(format!("Worker error: {}", e)));
                    return;
                }
                Ok(Err(_)) => {
                    // timeout Ok, recv Err (channel closed)
                    circuit.record_failure();
                    pool.metrics().total_errors.fetch_add(1, Ordering::Relaxed);

                    let _ = tx.send(CandleStringChunk::text(
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

                    let _ = tx.send(CandleStringChunk::text("Request timeout".to_string()));
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
