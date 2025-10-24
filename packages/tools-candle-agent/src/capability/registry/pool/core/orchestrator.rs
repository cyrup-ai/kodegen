// orchestrator.rs - Complete worker lifecycle orchestration

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tracing::{info, warn, error, instrument, span, Level};
use prometheus::core::{Collector, Desc};

use super::worker_state::{WorkerState, UnifiedWorkerHandle, CircuitBreaker, CircuitBreakerConfig};
use super::request_queue::{RequestQueue, PriorityRequest};
use super::memory_governor::{MemoryGovernor, MemoryPressure};
use super::PoolError;

/// Orchestrates worker lifecycle across all models
pub struct WorkerOrchestrator<Req: Send + 'static, Resp: Send + 'static> {
    /// All workers indexed by registry_key -> Vec<UnifiedWorkerHandle>
    workers: Arc<DashMap<String, Vec<Arc<UnifiedWorkerHandle<Req, Resp>>>>>,
    
    /// Request queues per model with priority and coalescing
    request_queues: Arc<DashMap<String, Arc<RequestQueue<Req>>>>,
    
    /// Circuit breakers per model
    circuit_breakers: Arc<DashMap<String, Arc<CircuitBreaker>>>,
    
    /// Memory governor for system-wide memory management
    memory_governor: Arc<MemoryGovernor>,
    
    /// Load predictor for adaptive scaling
    load_predictor: Arc<LoadPredictor>,
    
    /// Worker lifecycle callbacks
    callbacks: Arc<RwLock<LifecycleCallbacks<Req, Resp>>>,
    
    /// Orchestrator configuration
    config: OrchestratorConfig,
    
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
    
    /// Global worker ID counter
    next_worker_id: Arc<AtomicU64>,
    
    /// Background task handles
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

#[derive(Clone)]
pub struct OrchestratorConfig {
    pub max_workers_per_model: usize,
    pub min_workers_per_model: usize,
    pub spawn_timeout: Duration,
    pub health_check_interval: Duration,
    pub eviction_check_interval: Duration,
    pub scale_check_interval: Duration,
    pub request_timeout: Duration,
    pub idle_threshold: Duration,
    pub memory_limit_percent: f64,
    pub cpu_affinity: bool,
    pub numa_aware: bool,
    pub enable_work_stealing: bool,
    pub enable_request_coalescing: bool,
    pub enable_predictive_scaling: bool,
    pub enable_chaos_testing: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_workers_per_model: 8,
            min_workers_per_model: 0,
            spawn_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            eviction_check_interval: Duration::from_secs(60),
            scale_check_interval: Duration::from_secs(30),
            request_timeout: Duration::from_secs(60),
            idle_threshold: Duration::from_secs(300),
            memory_limit_percent: 0.80,
            cpu_affinity: true,
            numa_aware: true,
            enable_work_stealing: true,
            enable_request_coalescing: true,
            enable_predictive_scaling: true,
            enable_chaos_testing: false,
        }
    }
}

/// Lifecycle callbacks for extensibility
pub struct LifecycleCallbacks<Req, Resp> {
    pub on_worker_spawn: Option<Box<dyn Fn(u64, &str) + Send + Sync>>,
    pub on_worker_ready: Option<Box<dyn Fn(u64, &str) + Send + Sync>>,
    pub on_worker_evict: Option<Box<dyn Fn(u64, &str, &str) + Send + Sync>>,
    pub on_worker_fail: Option<Box<dyn Fn(u64, &str, &str) + Send + Sync>>,
    pub on_request_start: Option<Box<dyn Fn(&Req) + Send + Sync>>,
    pub on_request_complete: Option<Box<dyn Fn(&Resp, Duration) + Send + Sync>>,
    pub on_request_fail: Option<Box<dyn Fn(&str) + Send + Sync>>,
}

impl<Req: Send + 'static, Resp: Send + 'static> WorkerOrchestrator<Req, Resp> {
    pub fn new(config: OrchestratorConfig) -> Self {
        let memory_governor = Arc::new(MemoryGovernor::new(config.memory_limit_percent));
        let load_predictor = Arc::new(LoadPredictor::new());
        
        let orchestrator = Self {
            workers: Arc::new(DashMap::new()),
            request_queues: Arc::new(DashMap::new()),
            circuit_breakers: Arc::new(DashMap::new()),
            memory_governor: memory_governor.clone(),
            load_predictor: load_predictor.clone(),
            callbacks: Arc::new(RwLock::new(LifecycleCallbacks {
                on_worker_spawn: None,
                on_worker_ready: None,
                on_worker_evict: None,
                on_worker_fail: None,
                on_request_start: None,
                on_request_complete: None,
                on_request_fail: None,
            })),
            config: config.clone(),
            shutdown: Arc::new(AtomicBool::new(false)),
            next_worker_id: Arc::new(AtomicU64::new(1)),
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        };
        
        // Start background tasks
        orchestrator.start_background_tasks();
        
        orchestrator
    }
    
    fn start_background_tasks(&self) {
        let mut tasks = self.background_tasks.blocking_write();
        
        // Health monitor task
        let health_task = {
            let workers = self.workers.clone();
            let interval = self.config.health_check_interval;
            let shutdown = self.shutdown.clone();
            
            tokio::spawn(async move {
                while !shutdown.load(Ordering::Acquire) {
                    Self::run_health_checks(&workers).await;
                    tokio::time::sleep(interval).await;
                }
            })
        };
        tasks.push(health_task);
        
        // Eviction task
        let eviction_task = {
            let orchestrator = self.clone();
            let interval = self.config.eviction_check_interval;
            let shutdown = self.shutdown.clone();
            
            tokio::spawn(async move {
                while !shutdown.load(Ordering::Acquire) {
                    orchestrator.evict_idle_workers().await;
                    tokio::time::sleep(interval).await;
                }
            })
        };
        tasks.push(eviction_task);
        
        // Adaptive scaling task
        if self.config.enable_predictive_scaling {
            let scale_task = {
                let orchestrator = self.clone();
                let interval = self.config.scale_check_interval;
                let shutdown = self.shutdown.clone();
                
                tokio::spawn(async move {
                    while !shutdown.load(Ordering::Acquire) {
                        orchestrator.adaptive_scale().await;
                        tokio::time::sleep(interval).await;
                    }
                })
            };
            tasks.push(scale_task);
        }
    }
    
    #[instrument(skip(self, model_loader))]
    pub async fn spawn_worker<F, T>(
        &self,
        registry_key: &str,
        memory_mb: usize,
        model_loader: F,
    ) -> Result<Arc<UnifiedWorkerHandle<Req, Resp>>, PoolError>
    where
        F: FnOnce() -> Result<T, PoolError> + Send + 'static,
        T: Send + 'static,
    {
        // Check memory availability
        if !self.memory_governor.try_allocate(memory_mb).await {
            return Err(PoolError::MemoryExhausted(
                format!("Cannot allocate {} MB for {}", memory_mb, registry_key)
            ));
        }
        
        let worker_id = self.next_worker_id.fetch_add(1, Ordering::Relaxed);
        
        // Create channels with tokio mpsc
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (priority_tx, priority_rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
        let (health_tx, health_rx) = mpsc::unbounded_channel();
        let (health_status_tx, health_status_rx) = mpsc::unbounded_channel();
        
        // Create worker handle
        let worker = Arc::new(UnifiedWorkerHandle {
            worker_id,
            registry_key: registry_key.to_string(),
            state: Arc::new(AtomicU32::new(WorkerState::Spawning as u32)),
            pending_requests: Arc::new(AtomicU64::new(0)),
            processed_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            total_latency_us: Arc::new(AtomicU64::new(0)),
            last_activity: Arc::new(AtomicU64::new(0)),
            spawn_time: Instant::now(),
            memory_mb,
            cpu_cores: None,
            request_tx: request_tx.clone(),
            response_rx,
            priority_tx: priority_tx.clone(),
            shutdown_tx,
            health_tx,
            health_rx: health_status_rx,
            metrics: WorkerMetrics::new(worker_id, registry_key),
            circuit_breaker: self.get_or_create_circuit_breaker(registry_key),
            steal_handle: None,
        });
        
        // Spawn worker thread with full lifecycle management
        let worker_clone = worker.clone();
        let memory_governor = self.memory_governor.clone();
        let callbacks = self.callbacks.clone();
        
        std::thread::Builder::new()
            .name(format!("worker-{}-{}", registry_key, worker_id))
            .spawn(move || {
                // Set CPU affinity if enabled
                #[cfg(target_os = "linux")]
                if let Some(cores) = &worker_clone.cpu_cores {
                    set_cpu_affinity(cores);
                }
                
                // Transition to Loading state
                let _ = worker_clone.transition_state(WorkerState::Loading);
                
                // Load model
                let model = match model_loader() {
                    Ok(m) => {
                        info!("Worker {} loaded model successfully", worker_id);
                        let _ = worker_clone.transition_state(WorkerState::Ready);
                        m
                    }
                    Err(e) => {
                        error!("Worker {} failed to load model: {}", worker_id, e);
                        let _ = worker_clone.transition_state(WorkerState::Failed);
                        
                        // Clean up allocated memory
                        memory_governor.release(memory_mb);
                        
                        // Call failure callback
                        if let Some(callback) = &callbacks.blocking_read().on_worker_fail {
                            callback(worker_id, registry_key, &e.to_string());
                        }
                        
                        return;
                    }
                };
                
                // Run worker loop
                Self::worker_loop(
                    worker_clone,
                    model,
                    request_rx,
                    priority_rx,
                    response_tx,
                    shutdown_rx,
                    health_rx,
                    health_status_tx,
                );
                
                // Clean up on exit
                memory_governor.release(memory_mb);
            })
            .map_err(|e| PoolError::SpawnFailed(e.to_string()))?;
        
        // Register worker
        self.workers
            .entry(registry_key.to_string())
            .or_insert_with(Vec::new)
            .push(worker.clone());
        
        // Call spawn callback
        if let Some(callback) = &self.callbacks.read().await.on_worker_spawn {
            callback(worker_id, registry_key);
        }
        
        Ok(worker)
    }
    
    fn get_or_create_circuit_breaker(&self, registry_key: &str) -> Arc<CircuitBreaker> {
        self.circuit_breakers
            .entry(registry_key.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::new(CircuitBreakerConfig {
                    failure_threshold: 5,
                    success_threshold: 3,
                    timeout: Duration::from_secs(60),
                    half_open_requests: 3,
                }))
            })
            .clone()
    }
}