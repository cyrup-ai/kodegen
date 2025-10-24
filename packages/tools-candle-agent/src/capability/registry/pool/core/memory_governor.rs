// memory_governor.rs - System-wide memory management and pressure handling

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::RwLock;

/// Eviction candidate for emergency memory recovery
#[derive(Debug, Clone)]
pub struct EvictionCandidate {
    pub registry_key: String,
    pub worker_id: u64,
    pub size_mb: usize,
}

/// Memory allocation errors
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Memory exhausted: requested {requested} MB, only {available} MB available")]
    Exhausted { requested: usize, available: usize },

    #[error("Memory allocation requires eviction")]
    RequiresEviction(Vec<EvictionCandidate>),
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPressure {
    Low,      // < 50% usage
    Normal,   // 50-70% usage
    High,     // 70-85% usage
    Critical, // > 85% usage
}

/// System-wide memory governor
pub struct MemoryGovernor {
    /// Total system memory in MB
    total_system_mb: AtomicU64,

    /// Current allocated memory in MB
    allocated_mb: AtomicU64,

    /// Memory limit in MB (e.g., 80% of system)
    limit_mb: AtomicU64,

    /// Reserved memory for system in MB
    reserved_mb: AtomicU64,

    /// Current memory pressure
    pressure: Arc<RwLock<MemoryPressure>>,

    /// Memory allocations by model
    allocations: Arc<RwLock<BTreeMap<String, ModelMemory>>>,

    /// Memory pools for different sizes
    memory_pools: Arc<RwLock<MemoryPools>>,

    /// Queue of workers marked for emergency eviction
    eviction_queue: Arc<RwLock<Vec<EvictionCandidate>>>,

    /// Configuration
    config: MemoryConfig,

    /// System info handle
    system: Arc<RwLock<System>>,
}

/// RAII guard that automatically releases memory allocation on drop
///
/// Prevents memory leaks when worker spawning fails or panics.
/// Follows the same pattern as SpawnGuard in types.rs.
pub struct AllocationGuard {
    governor: MemoryGovernor,
    size_mb: usize,
}

impl Drop for AllocationGuard {
    fn drop(&mut self) {
        // Atomic release - can't fail
        let previous = self
            .governor
            .allocated_mb
            .fetch_sub(self.size_mb as u64, Ordering::Release);

        log::info!(
            "Released {} MB via AllocationGuard (total: {} MB)",
            self.size_mb,
            previous - self.size_mb as u64
        );

        // Runtime-aware cleanup to handle both async and sync contexts
        let governor = self.governor.clone();
        let size_mb = self.size_mb;

        // Try to detect if we're in a tokio runtime
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // In tokio runtime - spawn async cleanup
            handle.spawn(async move {
                // Update pressure (async version)
                governor.update_pressure().await;

                // Return memory to pool if enabled (async version)
                if governor.config.enable_memory_pools {
                    governor.return_to_pool(size_mb).await;
                }
            });
        } else {
            // Not in tokio runtime - use blocking versions
            governor.update_pressure_blocking();
            if governor.config.enable_memory_pools {
                governor.return_to_pool_blocking(size_mb);
            }
        }
    }
}

#[derive(Clone)]
pub struct ModelMemory {
    pub model_name: String,
    pub workers: usize,
    pub per_worker_mb: usize,
    pub total_mb: usize,
    pub last_accessed: Instant,
    pub allocations: Vec<AllocationInfo>,
}

#[derive(Clone)]
pub struct AllocationInfo {
    pub worker_id: u64,
    pub size_mb: usize,
    pub allocated_at: Instant,
    pub numa_node: Option<usize>,
    pub huge_pages: bool,
}

pub struct MemoryPools {
    small_pool: Vec<MemoryChunk>,  // < 100 MB
    medium_pool: Vec<MemoryChunk>, // 100-1000 MB
    large_pool: Vec<MemoryChunk>,  // > 1000 MB
}

#[derive(Clone)]
pub struct MemoryChunk {
    size_mb: usize,
    allocated: bool,
    last_used: Instant,
}

#[derive(Clone)]
pub struct MemoryConfig {
    pub memory_limit_percent: f64,
    pub reserved_system_mb: usize,
    pub enable_huge_pages: bool,
    pub enable_numa_aware: bool,
    pub enable_memory_pools: bool,
    pub compaction_threshold: f64,
    pub pressure_check_interval: Duration,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            memory_limit_percent: 0.80,
            reserved_system_mb: 2048, // Reserve 2GB for system
            enable_huge_pages: true,
            enable_numa_aware: true,
            enable_memory_pools: true,
            compaction_threshold: 0.75, // Compact when 75% fragmented
            pressure_check_interval: Duration::from_secs(5),
        }
    }
}

impl MemoryGovernor {
    pub fn new(memory_limit_percent: f64) -> Self {
        let config = MemoryConfig {
            memory_limit_percent,
            ..Default::default()
        };
        Self::with_config(config)
    }

    pub fn with_config(config: MemoryConfig) -> Self {
        let mut system = System::new_all();
        system.refresh_memory();

        let total_system_mb = system.total_memory() / 1024 / 1024;
        let limit_mb = ((total_system_mb as f64) * config.memory_limit_percent) as u64;

        let governor = Self {
            total_system_mb: AtomicU64::new(total_system_mb),
            allocated_mb: AtomicU64::new(0),
            limit_mb: AtomicU64::new(limit_mb),
            reserved_mb: AtomicU64::new(config.reserved_system_mb as u64),
            pressure: Arc::new(RwLock::new(MemoryPressure::Low)),
            allocations: Arc::new(RwLock::new(BTreeMap::new())),
            memory_pools: Arc::new(RwLock::new(MemoryPools {
                small_pool: Vec::new(),
                medium_pool: Vec::new(),
                large_pool: Vec::new(),
            })),
            eviction_queue: Arc::new(RwLock::new(Vec::new())),
            config: config.clone(),
            system: Arc::new(RwLock::new(system)),
        };

        // Start pressure monitor
        governor.start_pressure_monitor();

        // Initialize memory pools if enabled (runtime-aware)
        if config.enable_memory_pools {
            let governor_clone = governor.clone();

            // Try to detect if we're in a tokio runtime
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                // In tokio runtime - spawn async initialization
                handle.spawn(async move {
                    governor_clone.initialize_memory_pools().await;
                });
            } else {
                // Not in tokio runtime - skip pool initialization
                // Pools will remain empty but won't cause panics
                log::warn!(
                    "MemoryGovernor created outside tokio runtime - memory pooling disabled"
                );
            }
        }

        governor
    }

    /// Try to allocate memory for a worker (synchronous with CAS loop)
    pub async fn try_allocate(&self, size_mb: usize) -> Result<AllocationGuard, MemoryError> {
        let mut current = self.allocated_mb.load(Ordering::Acquire);
        let limit = self.limit_mb.load(Ordering::Acquire);
        let reserved = self.reserved_mb.load(Ordering::Acquire);

        loop {
            // Check if allocation would exceed limit
            if current + size_mb as u64 > limit - reserved {
                // Try to find evictable memory
                if let Some(evictable) = self.find_evictable_memory(size_mb).await {
                    return Err(MemoryError::RequiresEviction(evictable));
                }
                return Err(MemoryError::Exhausted {
                    requested: size_mb,
                    available: (limit - reserved - current) as usize,
                });
            }

            // Try atomic compare-and-swap to reserve memory
            match self.allocated_mb.compare_exchange_weak(
                current,
                current + size_mb as u64,
                Ordering::Release, // Success: publish reservation
                Ordering::Acquire, // Failure: get updated value
            ) {
                Ok(_) => {
                    // Successfully reserved - return RAII guard
                    log::info!(
                        "Allocated {} MB (total: {} MB)",
                        size_mb,
                        current + size_mb as u64
                    );
                    self.update_pressure().await;

                    // Enable huge pages if configured
                    #[cfg(target_os = "linux")]
                    if self.config.enable_huge_pages && size_mb >= 100 {
                        self.enable_huge_pages_for_allocation(size_mb);
                    }

                    return Ok(AllocationGuard {
                        governor: self.clone(),
                        size_mb,
                    });
                }
                Err(actual) => {
                    // Another thread modified allocated_mb - retry with new value
                    current = actual;
                }
            }
        }
    }

    /// Find evictable workers to free up memory
    async fn find_evictable_memory(&self, needed_mb: usize) -> Option<Vec<EvictionCandidate>> {
        let allocations = self.allocations.read().await;
        let mut candidates = Vec::new();
        let mut freed_mb = 0;

        // Sort by last_used (LRU)
        let mut sorted: Vec<_> = allocations.values().collect();
        sorted.sort_by_key(|a| a.last_accessed);

        for alloc in sorted {
            if freed_mb >= needed_mb {
                break;
            }

            // Find idle workers in this model
            for worker_alloc in &alloc.allocations {
                candidates.push(EvictionCandidate {
                    registry_key: alloc.model_name.clone(),
                    worker_id: worker_alloc.worker_id,
                    size_mb: worker_alloc.size_mb,
                });
                freed_mb += worker_alloc.size_mb;

                if freed_mb >= needed_mb {
                    return Some(candidates);
                }
            }
        }

        if freed_mb >= needed_mb {
            Some(candidates)
        } else {
            None
        }
    }

    /// Release allocated memory (manual release, prefer AllocationGuard)
    pub fn release(&self, size_mb: usize) {
        let previous = self
            .allocated_mb
            .fetch_sub(size_mb as u64, Ordering::Release);
        log::info!(
            "Released {} MB (total: {} MB)",
            size_mb,
            previous - size_mb as u64
        );

        // Return to pool if enabled (blocking version for sync context)
        if self.config.enable_memory_pools {
            self.return_to_pool_blocking(size_mb);
        }

        // Update pressure (blocking version for sync context)
        self.update_pressure_blocking();
    }

    /// Release allocated memory asynchronously (preferred in async contexts)
    pub async fn release_async(&self, size_mb: usize) {
        let previous = self
            .allocated_mb
            .fetch_sub(size_mb as u64, Ordering::Release);
        log::info!(
            "Released {} MB (total: {} MB)",
            size_mb,
            previous - size_mb as u64
        );

        // Return to pool if enabled (async version)
        if self.config.enable_memory_pools {
            self.return_to_pool(size_mb).await;
        }

        // Update pressure (async version)
        self.update_pressure().await;
    }

    /// Register model allocation
    pub async fn register_model_allocation(
        &self,
        model_name: &str,
        worker_id: u64,
        size_mb: usize,
    ) {
        // Get NUMA node before acquiring lock to avoid holding lock across await
        let numa_node = self.get_numa_node().await;

        let mut allocations = self.allocations.write().await;

        let allocation = AllocationInfo {
            worker_id,
            size_mb,
            allocated_at: Instant::now(),
            numa_node,
            huge_pages: self.config.enable_huge_pages,
        };

        allocations
            .entry(model_name.to_string())
            .and_modify(|m| {
                m.workers += 1;
                m.total_mb += size_mb;
                m.last_accessed = Instant::now();
                m.allocations.push(allocation.clone());
            })
            .or_insert_with(|| ModelMemory {
                model_name: model_name.to_string(),
                workers: 1,
                per_worker_mb: size_mb,
                total_mb: size_mb,
                last_accessed: Instant::now(),
                allocations: vec![allocation],
            });
    }

    /// Get current memory pressure
    pub async fn get_pressure(&self) -> MemoryPressure {
        *self.pressure.read().await
    }

    /// Get memory statistics
    pub async fn get_stats(&self) -> MemoryStats {
        let allocated = self.allocated_mb.load(Ordering::Acquire);
        let total = self.total_system_mb.load(Ordering::Acquire);
        let limit = self.limit_mb.load(Ordering::Acquire);

        MemoryStats {
            total_system_mb: total,
            allocated_mb: allocated,
            available_mb: limit.saturating_sub(allocated),
            limit_mb: limit,
            pressure: self.get_pressure().await,
            utilization: (allocated as f64) / (total as f64),
        }
    }

    /// Get models sorted by memory usage
    pub async fn get_models_by_memory(&self) -> Vec<ModelMemory> {
        let allocations = self.allocations.read().await;
        let mut models: Vec<_> = allocations.values().cloned().collect();
        models.sort_by(|a, b| b.total_mb.cmp(&a.total_mb));
        models
    }

    /// Suggest models to evict under memory pressure
    pub async fn suggest_evictions(&self, target_mb: usize) -> Vec<String> {
        let models = self.get_models_by_memory().await;
        let mut eviction_list = Vec::new();
        let mut freed_mb = 0;

        // Start with least recently used models
        let mut lru_models = models.clone();
        lru_models.sort_by(|a, b| a.last_accessed.cmp(&b.last_accessed));

        for model in lru_models {
            if freed_mb >= target_mb {
                break;
            }

            eviction_list.push(model.model_name.clone());
            freed_mb += model.per_worker_mb; // Evict one worker at a time
        }

        eviction_list
    }

    /// Get pending eviction candidates
    pub async fn get_eviction_queue(&self) -> Vec<EvictionCandidate> {
        let mut queue = self.eviction_queue.write().await;
        let candidates = queue.clone();
        queue.clear();
        candidates
    }

    async fn update_pressure(&self) {
        let allocated = self.allocated_mb.load(Ordering::Acquire);
        let total = self.total_system_mb.load(Ordering::Acquire);
        let usage_percent = (allocated as f64) / (total as f64);

        let new_pressure = match usage_percent {
            p if p < 0.50 => MemoryPressure::Low,
            p if p < 0.70 => MemoryPressure::Normal,
            p if p < 0.85 => MemoryPressure::High,
            _ => MemoryPressure::Critical,
        };

        let mut pressure = self.pressure.write().await;
        if *pressure != new_pressure {
            log::info!(
                "Memory pressure changed: {:?} -> {:?}",
                *pressure,
                new_pressure
            );
            *pressure = new_pressure;
        }
    }

    fn update_pressure_blocking(&self) {
        let allocated = self.allocated_mb.load(Ordering::Acquire);
        let total = self.total_system_mb.load(Ordering::Acquire);
        let usage_percent = (allocated as f64) / (total as f64);

        let new_pressure = match usage_percent {
            p if p < 0.50 => MemoryPressure::Low,
            p if p < 0.70 => MemoryPressure::Normal,
            p if p < 0.85 => MemoryPressure::High,
            _ => MemoryPressure::Critical,
        };

        let mut pressure = self.pressure.blocking_write();
        if *pressure != new_pressure {
            log::info!(
                "Memory pressure changed: {:?} -> {:?}",
                *pressure,
                new_pressure
            );
            *pressure = new_pressure;
        }
    }

    fn start_pressure_monitor(&self) {
        let governor = self.clone();
        let interval = self.config.pressure_check_interval;

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;

                // Refresh system memory info
                {
                    let mut sys = governor.system.write().await;
                    sys.refresh_memory();
                    let _used = sys.used_memory() / 1024 / 1024;
                    let total = sys.total_memory() / 1024 / 1024;
                    governor.total_system_mb.store(total, Ordering::Release);
                }

                // Update pressure
                governor.update_pressure().await;

                // Handle critical pressure
                if governor.get_pressure().await == MemoryPressure::Critical {
                    governor.handle_critical_pressure().await;
                }
            }
        });
    }

    async fn handle_critical_pressure(&self) {
        log::warn!("CRITICAL memory pressure - initiating emergency eviction");

        // Target: free 10% of allocated memory
        let target_mb = (self.allocated_mb.load(Ordering::Acquire) / 10) as usize;

        if let Some(candidates) = self.find_evictable_memory(target_mb).await {
            for candidate in &candidates {
                log::warn!(
                    "Emergency evicting worker {} from {} ({} MB)",
                    candidate.worker_id,
                    candidate.registry_key,
                    candidate.size_mb
                );
            }

            // Store eviction candidates for pool to handle
            // (Pool maintenance thread will pick these up)
            let mut eviction_queue = self.eviction_queue.write().await;
            eviction_queue.extend(candidates);
        } else {
            log::error!("No evictable workers found despite critical pressure!");
        }
    }

    async fn initialize_memory_pools(&self) {
        // Pre-allocate memory chunks for pooling
        let mut pools = self.memory_pools.write().await;

        // Create small chunks (50 MB each)
        for _ in 0..10 {
            pools.small_pool.push(MemoryChunk {
                size_mb: 50,
                allocated: false,
                last_used: Instant::now(),
            });
        }

        // Create medium chunks (500 MB each)
        for _ in 0..5 {
            pools.medium_pool.push(MemoryChunk {
                size_mb: 500,
                allocated: false,
                last_used: Instant::now(),
            });
        }

        // Create large chunks (2000 MB each)
        for _ in 0..2 {
            pools.large_pool.push(MemoryChunk {
                size_mb: 2000,
                allocated: false,
                last_used: Instant::now(),
            });
        }
    }

    fn return_to_pool_blocking(&self, size_mb: usize) {
        let mut pools = self.memory_pools.blocking_write();

        let pool = if size_mb <= 100 {
            &mut pools.small_pool
        } else if size_mb <= 1000 {
            &mut pools.medium_pool
        } else {
            &mut pools.large_pool
        };

        // Mark chunk as available
        if let Some(chunk) = pool
            .iter_mut()
            .find(|c| c.allocated && c.size_mb == size_mb)
        {
            chunk.allocated = false;
            chunk.last_used = Instant::now();
        }
    }

    async fn return_to_pool(&self, size_mb: usize) {
        let mut pools = self.memory_pools.write().await;

        let pool = if size_mb <= 100 {
            &mut pools.small_pool
        } else if size_mb <= 1000 {
            &mut pools.medium_pool
        } else {
            &mut pools.large_pool
        };

        // Mark chunk as available
        if let Some(chunk) = pool
            .iter_mut()
            .find(|c| c.allocated && c.size_mb == size_mb)
        {
            chunk.allocated = false;
            chunk.last_used = Instant::now();
        }
    }

    async fn get_numa_node(&self) -> Option<usize> {
        #[cfg(target_os = "linux")]
        if self.config.enable_numa_aware {
            // Get current NUMA node
            tokio::fs::read_to_string("/proc/self/numa_node")
                .await
                .ok()
                .and_then(|s| s.trim().parse().ok())
        } else {
            None
        }

        #[cfg(not(target_os = "linux"))]
        None
    }
}

impl Clone for MemoryGovernor {
    fn clone(&self) -> Self {
        Self {
            total_system_mb: AtomicU64::new(self.total_system_mb.load(Ordering::Acquire)),
            allocated_mb: AtomicU64::new(self.allocated_mb.load(Ordering::Acquire)),
            limit_mb: AtomicU64::new(self.limit_mb.load(Ordering::Acquire)),
            reserved_mb: AtomicU64::new(self.reserved_mb.load(Ordering::Acquire)),
            pressure: self.pressure.clone(),
            allocations: self.allocations.clone(),
            memory_pools: self.memory_pools.clone(),
            eviction_queue: self.eviction_queue.clone(),
            config: self.config.clone(),
            system: self.system.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_system_mb: u64,
    pub allocated_mb: u64,
    pub available_mb: u64,
    pub limit_mb: u64,
    pub pressure: MemoryPressure,
    pub utilization: f64,
}
