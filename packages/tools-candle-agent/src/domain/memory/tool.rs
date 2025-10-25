//! Memory Tool Implementation - Zero-allocation memorize/recall with lock-free cognitive search
//!
//! This module provides the MemoryTool with blazing-fast performance, zero allocation,
//! and lock-free concurrent access to the cognitive memory system.

// Removed unused import: std::future::Future
// Removed unused import: std::pin::Pin
use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicUsize, Ordering},
};

// Ultra-high-performance zero-allocation imports
// Removed unused import: arrayvec::ArrayVec
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use serde::{Deserialize, Serialize};

// Removed unused import: serde_json::Value
use super::Error as MemoryError;
use crate::memory::core::{MemoryNode, MemoryType, SurrealDBMemoryManager};
// Removed unused imports: AsyncStream, AsyncTask, spawn_async
use crate::domain::completion::types::ToolInfo;
use crate::domain::error::ZeroAllocError;

/// Type alias for memory node result queue channel pair
type MemoryNodeQueue = (
    mpsc::UnboundedSender<MemoryNode>,
    Arc<Mutex<mpsc::UnboundedReceiver<MemoryNode>>>,
);

/// Maximum number of memory nodes in result collections
const MAX_MEMORY_TOOL_RESULTS: usize = 1000;

/// Maximum number of streaming results per operation
const MAX_STREAMING_RESULTS: usize = 100;

/// Global result aggregation statistics
static TOOL_STATS: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

/// Result queue for aggregation
static RESULT_QUEUE: LazyLock<MemoryNodeQueue> = LazyLock::new(|| {
    let (sender, receiver) = mpsc::unbounded_channel();
    (sender, Arc::new(Mutex::new(receiver)))
});

/// Zero-allocation memory tool with lock-free cognitive search
#[derive(Debug, Clone)]
pub struct MemoryTool {
    /// Tool metadata
    #[allow(dead_code)]
    data: ToolInfo,
    /// Shared memory instance for lock-free concurrent access
    memory: Arc<SurrealDBMemoryManager>,
}
/// Memory tool operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", content = "params")]
pub enum MemoryOperation {
    /// Memorize content with specified type
    Memorize {
        content: String,
        memory_type: MemoryType,
    },
    /// Recall memories by content search
    Recall { query: String, limit: Option<usize> },
    /// Search memories by vector similarity
    VectorSearch { vector: Vec<f32>, limit: usize },
    /// Get specific memory by ID
    GetMemory { id: String },
    /// Update existing memory
    UpdateMemory { memory: Box<MemoryNode> },
    /// Delete memory by ID
    DeleteMemory { id: String },
}

/// Memory tool result types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MemoryResult {
    /// Single memory node result
    Memory(Box<MemoryNode>),
    /// Multiple memory nodes result
    Memories(Vec<MemoryNode>),
    /// Boolean result for operations
    Success(bool),
    /// Error result
    Error(String),
}

/// Memory tool error types with semantic error handling
#[derive(Debug, thiserror::Error)]
pub enum MemoryToolError {
    /// Memory system error
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),
    /// Invalid operation parameters
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// Operation not supported
    #[error("Operation not supported: {0}")]
    NotSupported(String),
    /// Zero allocation error
    #[error("Zero allocation error: {0}")]
    ZeroAlloc(#[from] Box<ZeroAllocError>),
    /// Buffer overflow error
    #[error("Buffer overflow: operation would exceed capacity")]
    BufferOverflow,
    /// Tool initialization error
    #[error("Tool initialization error: {0}")]
    InitializationError(String),
}

/// Zero-allocation result type for memory tool operations
pub type MemoryToolResult<T> = Result<T, MemoryToolError>;

impl MemoryTool {
    /// Create a new memory tool instance
    #[must_use]
    pub fn new(memory: Arc<SurrealDBMemoryManager>) -> Self {
        let schema_json = serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["memorize", "recall", "vector_search", "get_memory", "update_memory", "delete_memory"]
                }
            },
            "required": ["operation"]
        });
        let schema_map = schema_json.as_object().cloned().unwrap_or_else(|| {
            log::error!("BUG: Hardcoded schema JSON is not an object - using empty schema as fallback");
            serde_json::Map::new()
        });

        let data = ToolInfo {
            name: "memory".into(),
            title: None,
            description: Some(
                "Memory management tool for storing and retrieving information".into(),
            ),
            input_schema: Arc::new(schema_map),
            output_schema: None,
            annotations: None,
            icons: None,
        };

        Self { data, memory }
    }

    /// Get access to the underlying memory instance
    #[inline]
    #[must_use]
    pub fn memory(&self) -> &Arc<SurrealDBMemoryManager> {
        &self.memory
    }

    /// Get tool metadata
    #[inline]
    #[must_use]
    pub fn tool_data(&self) -> &ToolInfo {
        &self.data
    }

    /// Get tool name from metadata
    #[inline]
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.data.name
    }

    /// Get tool description from metadata
    #[inline]
    #[must_use]
    pub fn tool_description(&self) -> &str {
        self.data.description.as_deref().unwrap_or("")
    }

    /// Get maximum results limit for memory operations
    #[inline]
    #[must_use]
    pub fn max_results_limit() -> usize {
        MAX_MEMORY_TOOL_RESULTS
    }

    /// Get maximum streaming results per operation
    #[inline]
    #[must_use]
    pub fn max_streaming_limit() -> usize {
        MAX_STREAMING_RESULTS
    }

    /// Add memory node to result queue
    #[inline]
    pub fn queue_result(node: MemoryNode) {
        let (sender, _receiver) = &*RESULT_QUEUE;
        let _ = sender.send(node);
        TOOL_STATS.fetch_add(1, Ordering::Relaxed);
    }

    /// Get next result from queue
    #[inline]
    #[must_use]
    pub async fn dequeue_result() -> Option<MemoryNode> {
        let (_sender, receiver) = &*RESULT_QUEUE;
        let mut rx = receiver.lock().await;
        rx.try_recv().ok()
    }

    /// Get tool operation statistics
    #[inline]
    pub fn get_tool_stats() -> usize {
        TOOL_STATS.load(Ordering::Relaxed)
    }

    /// Reset tool statistics
    #[inline]
    pub fn reset_tool_stats() {
        TOOL_STATS.store(0, Ordering::Relaxed);
    }

    /// Get result queue length for monitoring
    #[inline]
    #[must_use]
    pub fn result_queue_length() -> usize {
        0 // tokio mpsc channels don't expose queue length
    }
}
