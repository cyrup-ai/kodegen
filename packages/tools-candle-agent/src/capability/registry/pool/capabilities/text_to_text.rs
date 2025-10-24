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
use crate::capability::traits::TextToTextCapable;
use crate::domain::completion::CandleCompletionParams;
use crate::domain::context::chunks::CandleCompletionChunk;
use crate::domain::prompt::CandlePrompt;

/// Type alias for text completion streaming response sender
type CompletionResponse =
    oneshot::Sender<Result<Pin<Box<dyn Stream<Item = CandleCompletionChunk> + Send>>, PoolError>>;

/// Request for prompt() operation (streaming response)
pub struct PromptRequest {
    pub prompt: CandlePrompt,
    pub params: CandleCompletionParams,
    pub response: CompletionResponse,
}

/// TextToText-specific worker handle with channel
#[derive(Clone)]
pub struct TextToTextWorkerHandle {
    pub core: WorkerHandle,
    pub prompt_tx: mpsc::UnboundedSender<PromptRequest>,
    pub shutdown_tx: mpsc::UnboundedSender<()>,
    pub registry_key: String, // Added to enable cleanup on drop
}

impl crate::capability::registry::pool::core::types::PoolWorkerHandle for TextToTextWorkerHandle {
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

impl std::ops::Deref for TextToTextWorkerHandle {
    type Target = WorkerHandle;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// Channels used by text to text worker
pub struct TextToTextWorkerChannels {
    pub prompt_rx: mpsc::UnboundedReceiver<PromptRequest>,
    pub shutdown_rx: mpsc::UnboundedReceiver<()>,
    pub health_rx: mpsc::UnboundedReceiver<HealthPing>,
    pub health_tx: mpsc::UnboundedSender<HealthPong>,
}

/// Context for text to text worker
pub struct TextToTextWorkerContext {
    pub worker_id: usize,
    pub registry_key: String,
    pub state: Arc<AtomicU32>,
}

/// Worker loop for TextToText models
///
/// Processes streaming prompt requests. Worker calls trait method which
/// returns Pin<Box<dyn Stream<Item = CandleCompletionChunk> + Send>>. Stream is sent back to caller
/// who forwards chunks to end user.
pub async fn text_to_text_worker<T: TextToTextCapable>(
    model: T,
    channels: TextToTextWorkerChannels,
    context: TextToTextWorkerContext,
) {
    use crate::capability::registry::pool::core::worker_state::WorkerState;
    use std::time::Duration;

    // Destructure channels and context
    let TextToTextWorkerChannels {
        mut prompt_rx,
        mut shutdown_rx,
        mut health_rx,
        health_tx,
    } = channels;
    let TextToTextWorkerContext {
        worker_id,
        registry_key: _registry_key,
        state,
    } = context;

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
            Some(req) = prompt_rx.recv() => {
                log::info!(">>> Worker {} received prompt request", worker_id);
                // Transition: Ready/Idle → Processing
                state.store(WorkerState::Processing as u32, std::sync::atomic::Ordering::Release);

                // Model method returns tokio Stream directly
                log::info!(">>> Worker {} calling model.prompt()", worker_id);
                let stream = model.prompt(req.prompt, &req.params);
                log::info!(">>> Worker {} got stream from model.prompt(), sending to response channel", worker_id);
                let _ = req.response.send(Ok(stream));

                // Transition: Processing → Ready
                state.store(WorkerState::Ready as u32, std::sync::atomic::Ordering::Release);
                last_activity = SystemTime::now();
                log::info!(">>> Worker {} completed request", worker_id);
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
                log::info!("TextToText worker {} shutting down", worker_id);
                // Transition: Ready/Idle → Evicting
                state.store(WorkerState::Evicting as u32, std::sync::atomic::Ordering::Release);
                break;
            }
        }
    }
}

/// Global TextToText pool instance
static TEXT_TO_TEXT_POOL: Lazy<Pool<TextToTextWorkerHandle>> =
    Lazy::new(|| Pool::new(PoolConfig::default()));

/// Access global TextToText pool
pub fn text_to_text_pool() -> &'static Pool<TextToTextWorkerHandle> {
    &TEXT_TO_TEXT_POOL
}

impl Pool<TextToTextWorkerHandle> {
    /// Spawn worker for TextToText model
    pub fn spawn_text_to_text_worker<T, F, Fut>(
        &self,
        registry_key: &str,
        model_loader: F,
        per_worker_mb: usize,
        allocation_guard: AllocationGuard,
    ) -> Result<(), PoolError>
    where
        T: TextToTextCapable + Send + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, PoolError>> + Send + 'static,
    {
        // Create unbounded channels for worker communication
        let (prompt_tx, prompt_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
        let (health_tx_main, health_rx_worker) = mpsc::unbounded_channel();
        let (health_tx_worker, health_rx_main) = mpsc::unbounded_channel();

        // Get worker ID before moving into thread
        let worker_id = self.next_worker_id();
        let registry_key_clone = registry_key.to_string();
        let registry_key_for_handle = registry_key.to_string();
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

            // Load model
            let model = match model_loader().await {
                Ok(m) => {
                    log::info!("TextToText worker {} ready", worker_id);
                    // Transition: Loading → Ready
                    state_clone.store(
                        WorkerState::Ready as u32,
                        std::sync::atomic::Ordering::Release,
                    );
                    m
                }
                Err(e) => {
                    log::error!("TextToText worker {} failed: {}", worker_id, e);
                    // Transition: Loading → Failed
                    state_clone.store(
                        WorkerState::Failed as u32,
                        std::sync::atomic::Ordering::Release,
                    );

                    // Clean up memory tracking
                    // This prevents memory leak when model loading fails
                    text_to_text_pool().remove_memory(per_worker_mb_clone);

                    return; // Exit task without running worker loop
                }
            };

            text_to_text_worker(
                model,
                TextToTextWorkerChannels {
                    prompt_rx,
                    shutdown_rx,
                    health_rx: health_rx_worker,
                    health_tx: health_tx_worker,
                },
                TextToTextWorkerContext {
                    worker_id,
                    registry_key: registry_key_clone.clone(),
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
        let full_handle = TextToTextWorkerHandle {
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
            prompt_tx,
            shutdown_tx,
            registry_key: registry_key_for_handle.clone(),
        };

        // Single registration point - no duplication
        self.register_worker(registry_key.to_string(), full_handle);

        // Update memory tracking
        self.add_memory(per_worker_mb);

        Ok(())
    }

    /// Generate completion using pooled worker (returns stream)
    pub fn prompt(
        &self,
        registry_key: &str,
        prompt: CandlePrompt,
        params: CandleCompletionParams,
    ) -> Pin<Box<dyn Stream<Item = CandleCompletionChunk> + Send>> {
        log::info!(
            ">>> TextToTextPool::prompt() called for registry_key={}, prompt_len={}",
            registry_key,
            prompt.content.len()
        );

        // Clone for move into closure
        let registry_key = registry_key.to_string();
        let is_shutting_down = self.is_shutting_down();
        let request_timeout_secs = self.config().request_timeout_secs;

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            log::info!(">>> Pool stream spawned for {}", registry_key);
            // Check shutdown
            if is_shutting_down {
                let _ = tx.send(CandleCompletionChunk::Error(
                    "Pool shutting down".to_string(),
                ));
                return;
            }

            // Get circuit breaker for this model and check state
            let pool = text_to_text_pool();
            let circuit = pool.get_circuit_breaker(&registry_key);

            if !circuit.can_request() {
                let _ = tx.send(CandleCompletionChunk::Error(format!(
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
                    let _ = tx.send(CandleCompletionChunk::Error(format!(
                        "No workers for {}",
                        registry_key
                    )));
                    return;
                }
            };

            if workers.is_empty() {
                let _ = tx.send(CandleCompletionChunk::Error(
                    "No workers available".to_string(),
                ));
                return;
            }

            // Find alive worker with least load using Power of Two Choices (O(1))
            let alive_workers: Vec<_> = workers.iter().filter(|w| w.core.is_alive()).collect();

            let worker = match select_worker_power_of_two(&alive_workers, |w| &w.core) {
                Some(w) => w,
                None => {
                    let _ = tx.send(CandleCompletionChunk::Error(format!(
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
            if let Err(e) = worker.prompt_tx.send(PromptRequest {
                prompt,
                params,
                response: response_tx,
            }) {
                let _ = tx.send(CandleCompletionChunk::Error(format!(
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

                    let _ = tx.send(CandleCompletionChunk::Error(format!("Worker error: {}", e)));
                    return;
                }
                Ok(Err(_)) => {
                    // timeout Ok, recv Err (channel closed)
                    circuit.record_failure();
                    pool.metrics().total_errors.fetch_add(1, Ordering::Relaxed);

                    let _ = tx.send(CandleCompletionChunk::Error(
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

                    let _ = tx.send(CandleCompletionChunk::Error("Request timeout".to_string()));
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
