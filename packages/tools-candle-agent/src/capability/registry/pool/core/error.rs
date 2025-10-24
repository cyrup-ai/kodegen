use std::fmt;

#[derive(Debug, Clone)]
pub enum PoolError {
    NoWorkers(String),       // No workers spawned for registry_key
    Timeout(String),         // Request timed out after N seconds
    SendError(String),       // Failed to send request to worker
    RecvError(String),       // Failed to receive response from worker
    ModelError(String),      // Model inference error
    ShuttingDown(String),    // Pool shutting down, rejecting requests
    MemoryExhausted(String), // Cannot spawn worker, 80% limit reached
    SpawnFailed(String),     // Worker thread spawn failed
    SpawnTimeout(String),    // Timeout waiting for another thread to spawn workers
    CircuitOpen(String),     // Circuit breaker open, rejecting requests
    RuntimeUnavailable,      // Shared runtime not available for async operations
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoWorkers(msg) => write!(f, "No workers available: {}", msg),
            Self::Timeout(msg) => write!(f, "Request timeout: {}", msg),
            Self::SendError(msg) => write!(f, "Channel send error: {}", msg),
            Self::RecvError(msg) => write!(f, "Channel recv error: {}", msg),
            Self::ModelError(msg) => write!(f, "Model error: {}", msg),
            Self::ShuttingDown(msg) => write!(f, "Shutting down: {}", msg),
            Self::MemoryExhausted(msg) => write!(f, "Memory exhausted: {}", msg),
            Self::SpawnFailed(msg) => write!(f, "Worker spawn failed: {}", msg),
            Self::SpawnTimeout(msg) => write!(f, "Spawn timeout: {}", msg),
            Self::CircuitOpen(msg) => write!(f, "Circuit breaker open: {}", msg),
            Self::RuntimeUnavailable => write!(f, "Shared runtime unavailable"),
        }
    }
}

impl std::error::Error for PoolError {}
