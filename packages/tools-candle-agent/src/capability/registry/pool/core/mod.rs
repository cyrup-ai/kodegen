pub mod error;
pub mod memory;
pub mod memory_governor;
pub mod pool;
pub mod spawn;
pub mod types;
pub mod worker;
pub mod worker_state;

pub use error::PoolError;
pub use memory::query_system_memory_mb;
pub use memory_governor::{
    AllocationGuard, EvictionCandidate, MemoryError, MemoryGovernor, MemoryPressure,
};
pub use pool::Pool;
pub use spawn::{
    HasWorkers, MemoryGovernorAccess, SpawnLock, WorkerMetrics, ensure_workers_spawned,
    ensure_workers_spawned_adaptive,
};
pub use types::{PoolConfig, PoolMetrics, PoolWorkerHandle, SpawnGuard, WorkerHandle};
pub use worker::{check_memory_available, spawn_worker_thread};
pub use worker_state::{
    CircuitBreaker, HealthCheck, HealthStatus, UnifiedWorkerHandle, WorkerState,
};
