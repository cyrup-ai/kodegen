// request_queue.rs - Advanced request queueing with priority and coalescing

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Ordering as CmpOrdering;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::time::interval;
use dashmap::DashMap;
use xxhash_rust::xxh3::Xxh3;
use std::hash::{Hash, Hasher};

/// Request priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 0,  // System-critical requests
    High = 1,      // User-facing, latency-sensitive
    Normal = 2,    // Default priority
    Low = 3,       // Background tasks
    Batch = 4,     // Bulk operations, can be delayed
}

/// Request wrapper with priority and metadata
pub struct PriorityRequest<T> {
    pub id: u64,
    pub priority: Priority,
    pub request: T,
    pub enqueue_time: Instant,
    pub deadline: Option<Instant>,
    pub retry_count: u32,
    pub hash: Option<u64>,  // For deduplication
    pub batch_key: Option<String>,  // For coalescing
}

impl<T> PartialEq for PriorityRequest<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for PriorityRequest<T> {}

impl<T> PartialOrd for PriorityRequest<T> {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PriorityRequest<T> {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        // Higher priority first, then earlier enqueue time
        self.priority.cmp(&other.priority)
            .then_with(|| self.enqueue_time.cmp(&other.enqueue_time))
    }
}

/// Advanced request queue with multiple optimization strategies
pub struct RequestQueue<T: Send + 'static> {
    /// Priority queue for requests
    priority_queue: Arc<RwLock<BinaryHeap<Arc<PriorityRequest<T>>>>>,
    
    /// Deduplication cache (hash -> request)
    dedup_cache: Arc<DashMap<u64, Arc<PriorityRequest<T>>>>,
    
    /// Batch accumulator for coalescing
    batch_accumulator: Arc<DashMap<String, BatchAccumulator<T>>>,
    
    /// Request history for analytics
    request_history: Arc<RwLock<VecDeque<RequestStats>>>,
    
    /// Queue metrics
    metrics: Arc<QueueMetrics>,
    
    /// Configuration
    config: QueueConfig,
    
    /// Next request ID
    next_request_id: Arc<AtomicU64>,
}

pub struct BatchAccumulator<T> {
    pub requests: Vec<Arc<PriorityRequest<T>>>,
    pub first_arrival: Instant,
    pub batch_size: usize,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct RequestStats {
    pub id: u64,
    pub priority: Priority,
    pub queue_time_ms: f64,
    pub processing_time_ms: Option<f64>,
    pub success: bool,
}

pub struct QueueMetrics {
    pub total_enqueued: AtomicU64,
    pub total_dequeued: AtomicU64,
    pub total_deduplicated: AtomicU64,
    pub total_coalesced: AtomicU64,
    pub total_expired: AtomicU64,
    pub current_depth: AtomicUsize,
    pub max_depth: AtomicUsize,
}

#[derive(Clone)]
pub struct QueueConfig {
    pub max_queue_size: usize,
    pub enable_deduplication: bool,
    pub dedup_window: Duration,
    pub enable_coalescing: bool,
    pub coalesce_window: Duration,
    pub coalesce_max_batch: usize,
    pub enable_deadline_scheduling: bool,
    pub enable_fair_queuing: bool,
    pub history_size: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            enable_deduplication: true,
            dedup_window: Duration::from_millis(100),
            enable_coalescing: true,
            coalesce_window: Duration::from_millis(50),
            coalesce_max_batch: 32,
            enable_deadline_scheduling: true,
            enable_fair_queuing: true,
            history_size: 1000,
        }
    }
}

impl<T: Send + Clone + Hash + 'static> RequestQueue<T> {
    pub fn new(config: QueueConfig) -> Self {
        let queue = Self {
            priority_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            dedup_cache: Arc::new(DashMap::new()),
            batch_accumulator: Arc::new(DashMap::new()),
            request_history: Arc::new(RwLock::new(VecDeque::with_capacity(config.history_size))),
            metrics: Arc::new(QueueMetrics {
                total_enqueued: AtomicU64::new(0),
                total_dequeued: AtomicU64::new(0),
                total_deduplicated: AtomicU64::new(0),
                total_coalesced: AtomicU64::new(0),
                total_expired: AtomicU64::new(0),
                current_depth: AtomicUsize::new(0),
                max_depth: AtomicUsize::new(0),
            }),
            config: config.clone(),
            next_request_id: Arc::new(AtomicU64::new(1)),
        };
        
        // Start batch flush task if coalescing enabled
        if config.enable_coalescing {
            queue.start_batch_flush_task();
        }
        
        // Start deadline checker if enabled
        if config.enable_deadline_scheduling {
            queue.start_deadline_checker();
        }
        
        queue
    }
    
    /// Enqueue request with priority and optional batching
    pub async fn enqueue(
        &self,
        request: T,
        priority: Priority,
        deadline: Option<Duration>,
        batch_key: Option<String>,
    ) -> Result<u64, QueueError> {
        // Check queue capacity
        let current = self.metrics.current_depth.load(Ordering::Acquire);
        if current >= self.config.max_queue_size {
            return Err(QueueError::QueueFull);
        }
        
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let now = Instant::now();
        
        // Calculate hash for deduplication
        let hash = if self.config.enable_deduplication {
            let mut hasher = Xxh3::new();
            request.hash(&mut hasher);
            Some(hasher.finish())
        } else {
            None
        };
        
        // Check deduplication
        if let Some(hash_val) = hash {
            if let Some(existing) = self.dedup_cache.get(&hash_val) {
                let age = now.duration_since(existing.enqueue_time);
                if age < self.config.dedup_window {
                    self.metrics.total_deduplicated.fetch_add(1, Ordering::Relaxed);
                    return Ok(existing.id);  // Return existing request ID
                }
            }
        }
        
        // Create priority request
        let priority_req = Arc::new(PriorityRequest {
            id: request_id,
            priority,
            request: request.clone(),
            enqueue_time: now,
            deadline: deadline.map(|d| now + d),
            retry_count: 0,
            hash,
            batch_key: batch_key.clone(),
        });
        
        // Handle batching if enabled
        if self.config.enable_coalescing {
            if let Some(batch_key) = batch_key {
                self.batch_accumulator
                .entry(batch_key.clone())
                .and_modify(|acc| {
                    if acc.requests.len() < self.config.coalesce_max_batch {
                        acc.requests.push(priority_req.clone());
                        self.metrics.total_coalesced.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .or_insert_with(|| BatchAccumulator {
                    requests: vec![priority_req.clone()],
                    first_arrival: now,
                    batch_size: self.config.coalesce_max_batch,
                    timeout: self.config.coalesce_window,
                });
            } else {
                // No batch key - add to priority queue
                self.priority_queue.write().await.push(priority_req.clone());
            }
        } else {
            // Add to priority queue
            self.priority_queue.write().await.push(priority_req.clone());
        }
        
        // Update dedup cache
        if let Some(hash_val) = hash {
            self.dedup_cache.insert(hash_val, priority_req.clone());
        }
        
        // Update metrics
        self.metrics.total_enqueued.fetch_add(1, Ordering::Relaxed);
        self.metrics.current_depth.fetch_add(1, Ordering::Relaxed);
        self.update_max_depth(current + 1);
        
        Ok(request_id)
    }
    
    /// Dequeue highest priority request
    pub async fn dequeue(&self) -> Option<Arc<PriorityRequest<T>>> {
        let mut queue = self.priority_queue.write().await;
        
        // Check expired deadlines first
        if self.config.enable_deadline_scheduling {
            self.remove_expired_requests(&mut queue);
        }
        
        if let Some(request) = queue.pop() {
            // Update metrics
            self.metrics.total_dequeued.fetch_add(1, Ordering::Relaxed);
            self.metrics.current_depth.fetch_sub(1, Ordering::Relaxed);
            
            // Record stats
            let queue_time = request.enqueue_time.elapsed();
            self.record_request_stats(RequestStats {
                id: request.id,
                priority: request.priority,
                queue_time_ms: queue_time.as_millis() as f64,
                processing_time_ms: None,
                success: false,
            }).await;
            
            // Remove from dedup cache
            if let Some(hash) = request.hash {
                self.dedup_cache.remove(&hash);
            }
            
            Some(request)
        } else {
            None
        }
    }
    
    fn start_batch_flush_task(&self) {
        let accumulator = self.batch_accumulator.clone();
        let queue = self.priority_queue.clone();
        let window = self.config.coalesce_window;
        
        tokio::spawn(async move {
            let mut interval = interval(window / 2);
            loop {
                interval.tick().await;
                
                let now = Instant::now();
                let mut to_flush = Vec::new();
                
                // Find batches ready to flush
                for entry in accumulator.iter() {
                    let key = entry.key().clone();
                    let batch = entry.value();
                    
                    let age = now.duration_since(batch.first_arrival);
                    if age >= batch.timeout || batch.requests.len() >= batch.batch_size {
                        to_flush.push(key);
                    }
                }
                
                // Flush batches
                for key in to_flush {
                    if let Some((_, batch)) = accumulator.remove(&key) {
                        let mut queue = queue.write().await;
                        for req in batch.requests {
                            queue.push(req);
                        }
                    }
                }
            }
        });
    }
    
    fn start_deadline_checker(&self) {
        let queue = self.priority_queue.clone();
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                
                let mut queue = queue.write().await;
                let now = Instant::now();
                let mut expired = Vec::new();
                
                // Find expired requests
                for req in queue.iter() {
                    if let Some(deadline) = req.deadline {
                        if now > deadline {
                            expired.push(req.id);
                        }
                    }
                }
                
                // Remove expired
                if !expired.is_empty() {
                    let expired_count = expired.len();
                    queue.retain(|req| !expired.contains(&req.id));
                    metrics.total_expired.fetch_add(expired_count as u64, Ordering::Relaxed);
                }
            }
        });
    }
    
    fn remove_expired_requests(&self, queue: &mut BinaryHeap<Arc<PriorityRequest<T>>>) {
        let now = Instant::now();
        queue.retain(|req| {
            if let Some(deadline) = req.deadline {
                if now > deadline {
                    self.metrics.total_expired.fetch_add(1, Ordering::Relaxed);
                    return false;
                }
            }
            true
        });
    }
    
    fn update_max_depth(&self, depth: usize) {
        let mut max = self.metrics.max_depth.load(Ordering::Acquire);
        while depth > max {
            match self.metrics.max_depth.compare_exchange_weak(
                max,
                depth,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => max = current,
            }
        }
    }
    
    async fn record_request_stats(&self, stats: RequestStats) {
        let mut history = self.request_history.write().await;
        if history.len() >= self.config.history_size {
            history.pop_front();
        }
        history.push_back(stats);
    }
    
    pub fn get_stats(&self) -> QueueStats {
        QueueStats {
            current_depth: self.metrics.current_depth.load(Ordering::Acquire),
            max_depth: self.metrics.max_depth.load(Ordering::Acquire),
            total_enqueued: self.metrics.total_enqueued.load(Ordering::Acquire),
            total_dequeued: self.metrics.total_dequeued.load(Ordering::Acquire),
            total_deduplicated: self.metrics.total_deduplicated.load(Ordering::Acquire),
            total_coalesced: self.metrics.total_coalesced.load(Ordering::Acquire),
            total_expired: self.metrics.total_expired.load(Ordering::Acquire),
        }
    }
}

#[derive(Debug)]
pub struct QueueStats {
    pub current_depth: usize,
    pub max_depth: usize,
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_deduplicated: u64,
    pub total_coalesced: u64,
    pub total_expired: u64,
}

#[derive(Debug)]
pub enum QueueError {
    QueueFull,
    DeadlineExpired,
    RequestCancelled,
}