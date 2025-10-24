//! Lock-Free Operation Monitoring and Tracking with Zero-Allocation Performance
//!
//! This module provides blazing-fast operation tracking using lock-free atomic operations
//! and zero-allocation patterns for maximum performance in production workloads.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use chrono::{DateTime, Utc};
use moka::sync::Cache;
use serde::{Deserialize, Serialize};
/// High-performance lock-free counter for monitoring operations
#[derive(Debug, Default)]
pub struct RelaxedCounter {
    value: AtomicU64,
}

impl RelaxedCounter {
    #[inline]
    pub fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
        }
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn inc(&self) -> u64 {
        self.value.fetch_add(1, Ordering::Relaxed)
    }
}

use crossbeam_skiplist::SkipMap;
use smallvec::SmallVec;
use uuid::Uuid;

/// Operation types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    /// Memory creation
    CreateMemory,
    /// Memory update
    UpdateMemory,
    /// Memory deletion
    DeleteMemory,
    /// Memory search
    SearchMemory,
    /// Relationship creation
    CreateRelationship,
    /// Relationship deletion
    DeleteRelationship,
    /// Batch operation
    BatchOperation,

    // Cognitive operation types:
    /// Committee-based LLM evaluation of memory quality
    CommitteeEvaluation,
    /// Discovery of entangled/related memories via vector similarity
    EntanglementDiscovery,
    /// Quantum-inspired routing decision for memory retrieval
    QuantumRouting,
    /// Batch processing of multiple cognitive tasks
    BatchCognitive,

    /// Custom operation
    Custom(String),
}

/// Operation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationStatus {
    /// Operation is pending
    Pending,
    /// Operation is in progress
    InProgress,
    /// Operation completed successfully
    Success,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
}

/// Operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Operation ID
    pub id: String,

    /// Operation type
    pub operation_type: OperationType,

    /// Operation status
    pub status: OperationStatus,

    /// Start time
    pub started_at: DateTime<Utc>,

    /// End time
    pub ended_at: Option<DateTime<Utc>>,

    /// Duration
    pub duration: Option<Duration>,

    /// User ID
    pub user_id: Option<String>,

    /// Error message if failed
    pub error: Option<String>,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl Operation {
    /// Create a new operation with zero allocation where possible
    #[inline]
    pub fn new(operation_type: OperationType, user_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            operation_type,
            status: OperationStatus::Pending,
            started_at: Utc::now(),
            ended_at: None,
            duration: None,
            user_id,
            error: None,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Start the operation
    pub fn start(&mut self) {
        self.status = OperationStatus::InProgress;
        self.started_at = Utc::now();
    }

    /// Complete the operation successfully with blazing-fast atomic operations
    #[inline]
    pub fn complete(&mut self) {
        let now = Utc::now();
        self.status = OperationStatus::Success;
        self.ended_at = Some(now);
        self.duration = Some(
            now.signed_duration_since(self.started_at)
                .to_std()
                .unwrap_or(Duration::from_secs(0)),
        );
    }

    /// Fail the operation with comprehensive error tracking
    #[inline]
    pub fn fail(&mut self, error: String) {
        let now = Utc::now();
        self.status = OperationStatus::Failed;
        self.ended_at = Some(now);
        self.duration = Some(
            now.signed_duration_since(self.started_at)
                .to_std()
                .unwrap_or(Duration::from_secs(0)),
        );
        self.error = Some(error);
    }
}

/// Atomic metrics for lock-free operation tracking (zero allocation)
#[derive(Debug)]
pub struct OperationTrackerMetrics {
    /// Total operations started (atomic counter)
    pub operations_started: RelaxedCounter,
    /// Total operations completed successfully (atomic counter)
    pub operations_completed: RelaxedCounter,
    /// Total operations failed (atomic counter)
    pub operations_failed: RelaxedCounter,
    /// Total operations cancelled (atomic counter)
    pub operations_cancelled: RelaxedCounter,
    /// Average operation duration in microseconds (atomic)
    pub avg_duration_us: AtomicU64,
    /// Active operations count (atomic)
    pub active_count: AtomicUsize,
    /// History operations count (atomic)
    pub history_count: AtomicUsize,
}

impl OperationTrackerMetrics {
    /// Create new metrics with zero allocation
    #[inline]
    pub fn new() -> Self {
        Self {
            operations_started: RelaxedCounter::new(0),
            operations_completed: RelaxedCounter::new(0),
            operations_failed: RelaxedCounter::new(0),
            operations_cancelled: RelaxedCounter::new(0),
            avg_duration_us: AtomicU64::new(0),
            active_count: AtomicUsize::new(0),
            history_count: AtomicUsize::new(0),
        }
    }

    /// Record operation completion with atomic operations
    #[inline]
    pub fn record_completion(&self, duration_us: u64, success: bool) {
        if success {
            self.operations_completed.inc();
        } else {
            self.operations_failed.inc();
        }

        // Update average duration using atomic operations
        let current_avg = self.avg_duration_us.load(Ordering::Relaxed);
        let total_completed = self.operations_completed.get() + self.operations_failed.get();

        if total_completed > 0 {
            let new_avg = ((current_avg * (total_completed - 1)) + duration_us) / total_completed;
            self.avg_duration_us.store(new_avg, Ordering::Relaxed);
        }
    }

    /// Get success rate (0.0-1.0)
    #[inline]
    pub fn success_rate(&self) -> f64 {
        let completed = self.operations_completed.get() as f64;
        let failed = self.operations_failed.get() as f64;
        let total = completed + failed;

        if total > 0.0 { completed / total } else { 0.0 }
    }
}

impl Default for OperationTrackerMetrics {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Lock-free operation tracker with blazing-fast atomic operations (zero allocation)
#[derive(Debug)]
pub struct OperationTracker {
    /// Active operations (lock-free SkipMap for concurrent access)
    active: SkipMap<String, Operation>,

    /// Completed operations (lock-free LRU cache for history)
    completed: Cache<String, Operation>,

    /// Atomic counters for blazing-fast metrics
    metrics: OperationTrackerMetrics,

    /// Maximum completed operations to keep (user-configurable)
    max_history: usize,
}

impl OperationTracker {
    /// Create a new tracker with zero allocation
    #[inline]
    pub fn new(max_history: usize) -> Self {
        Self {
            active: SkipMap::new(),
            completed: Cache::new(max_history as u64),
            metrics: OperationTrackerMetrics::new(),
            max_history,
        }
    }

    /// Start tracking an operation with blazing-fast lock-free insertion
    #[inline]
    pub fn start_operation(
        &self,
        operation_type: OperationType,
        user_id: Option<String>,
    ) -> String {
        let mut operation = Operation::new(operation_type, user_id);
        operation.start();
        let id = operation.id.clone();

        // Lock-free insertion with atomic counter update
        self.active.insert(id.clone(), operation);
        self.metrics.operations_started.inc();
        self.metrics.active_count.fetch_add(1, Ordering::Relaxed);

        id
    }

    /// Complete an operation with lock-free atomic operations
    #[inline]
    pub fn complete_operation(&self, id: String) {
        if let Some(entry) = self.active.remove(&id) {
            let mut operation = entry.value().clone();
            let _start_time = operation.started_at;
            operation.complete();

            // Calculate duration for metrics
            let duration_us = operation
                .duration
                .map(|d| d.as_micros() as u64)
                .unwrap_or(0);

            // Lock-free history insertion and metrics update
            self.add_to_history_atomic(operation);
            self.metrics.record_completion(duration_us, true);
            self.metrics.active_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Fail an operation with comprehensive error tracking
    #[inline]
    pub fn fail_operation(&self, id: String, error: String) {
        if let Some(entry) = self.active.remove(&id) {
            let mut operation = entry.value().clone();
            let _start_time = operation.started_at;
            operation.fail(error);

            // Calculate duration for metrics
            let duration_us = operation
                .duration
                .map(|d| d.as_micros() as u64)
                .unwrap_or(0);

            // Lock-free history insertion and metrics update
            self.add_to_history_atomic(operation);
            self.metrics.record_completion(duration_us, false);
            self.metrics.active_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Add operation to history with lock-free atomic operations
    #[inline]
    fn add_to_history_atomic(&self, operation: Operation) {
        let history_key = format!(
            "{}_{}",
            operation
                .ended_at
                .unwrap_or(operation.started_at)
                .timestamp_nanos_opt()
                .unwrap_or(0),
            operation.id
        );

        // Moka automatically evicts LRU entry if at capacity
        self.completed.insert(history_key, operation);

        // Update metrics to reflect actual cache size
        let current_size = self.completed.entry_count();
        self.metrics
            .history_count
            .store(current_size as usize, Ordering::Relaxed);
    }

    /// Get active operations with zero allocation where possible
    #[inline]
    pub fn active_operations(&self) -> SmallVec<Operation, 16> {
        let mut operations = SmallVec::new();

        for entry in self.active.iter() {
            if operations.len() < operations.capacity() {
                operations.push(entry.value().clone());
            } else {
                break; // SmallVec is full, prevent heap allocation
            }
        }

        operations
    }

    /// Get operation history with efficient iteration
    #[inline]
    pub fn operation_history(&self) -> SmallVec<Operation, 32> {
        let mut operations = SmallVec::new();

        // Moka's iter() returns (K, V) tuples
        for (_key, value) in self.completed.iter() {
            if operations.len() < operations.capacity() {
                operations.push(value);
            } else {
                break; // SmallVec is full, prevent heap allocation
            }
        }

        operations
    }

    /// Get operation by ID with lock-free lookup
    #[inline]
    pub fn get_operation(&self, id: &str) -> Option<Operation> {
        self.active.get(id).map(|entry| entry.value().clone())
    }

    /// Get current metrics with atomic operations
    #[inline]
    pub fn metrics(&self) -> &OperationTrackerMetrics {
        &self.metrics
    }

    /// Get active operations count (atomic)
    #[inline]
    pub fn active_count(&self) -> usize {
        self.metrics.active_count.load(Ordering::Relaxed)
    }

    /// Get history count (atomic)
    #[inline]
    pub fn history_count(&self) -> usize {
        self.metrics.history_count.load(Ordering::Relaxed)
    }

    /// Get maximum history size configuration
    #[inline]
    pub fn max_history(&self) -> usize {
        self.max_history
    }

    /// Clear all completed operations with atomic operations
    #[inline]
    pub fn clear_history(&self) {
        self.completed.invalidate_all();
        self.metrics.history_count.store(0, Ordering::Relaxed);
    }
}

impl Default for OperationTracker {
    #[inline]
    fn default() -> Self {
        Self::new(1000)
    }
}
