use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

/// Cognitive processing queue with batching and prioritization
/// Task types for cognitive processing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveTaskType {
    /// Evaluate memory quality using Committee (LLM-based)
    CommitteeEvaluation,
    /// Determine routing strategy using QuantumRouter
    QuantumRouting,
    /// Discover entangled/related memories
    EntanglementDiscovery,
    /// Batch processing of multiple memory IDs
    BatchProcessing(Vec<String>),
}

/// Cognitive task with priority and metadata
#[derive(Debug, Clone)]
pub struct CognitiveTask {
    pub memory_id: String,
    pub task_type: CognitiveTaskType,
    pub priority: u8, // 0=lowest, 255=highest
    pub created_at: DateTime<Utc>,
}

impl CognitiveTask {
    pub fn new(memory_id: String, task_type: CognitiveTaskType, priority: u8) -> Self {
        Self {
            memory_id,
            task_type,
            priority,
            created_at: Utc::now(),
        }
    }
}

/// Accumulates tasks for efficient batch processing
#[derive(Debug)]
pub struct BatchAccumulator {
    tasks: Vec<CognitiveTask>,
    max_batch_size: usize,
    max_wait: Duration,
    last_batch_time: Instant,
}

impl BatchAccumulator {
    pub fn new(max_batch_size: usize, max_wait_ms: u64) -> Self {
        Self {
            tasks: Vec::with_capacity(max_batch_size),
            max_batch_size,
            max_wait: Duration::from_millis(max_wait_ms),
            last_batch_time: Instant::now(),
        }
    }

    /// Add task and return batch if ready (size or timeout reached)
    pub fn add(&mut self, task: CognitiveTask) -> Option<Vec<CognitiveTask>> {
        self.tasks.push(task);

        // Check if batch is ready
        let size_reached = self.tasks.len() >= self.max_batch_size;
        let timeout_reached = self.last_batch_time.elapsed() >= self.max_wait;

        if size_reached || (timeout_reached && !self.tasks.is_empty()) {
            let batch = std::mem::take(&mut self.tasks);
            self.last_batch_time = Instant::now();
            Some(batch)
        } else {
            None
        }
    }

    /// Force flush any pending tasks
    pub fn flush(&mut self) -> Option<Vec<CognitiveTask>> {
        if self.tasks.is_empty() {
            None
        } else {
            let batch = std::mem::take(&mut self.tasks);
            self.last_batch_time = Instant::now();
            Some(batch)
        }
    }
}

/// Async queue for cognitive processing
#[derive(Debug)]
pub struct CognitiveProcessingQueue {
    sender: UnboundedSender<CognitiveTask>,
    receiver: Arc<tokio::sync::Mutex<UnboundedReceiver<CognitiveTask>>>,
    /// Batch accumulator for CommitteeEvaluation tasks
    ///
    /// SYNC CONTEXT: Uses `std::sync::Mutex` because it's only accessed from
    /// non-async functions (`enqueue_with_batching`, `flush_batches`).
    /// These functions are intentionally synchronous to provide a blocking
    /// enqueue API alongside the async dequeue operations.
    batch_accumulator: Arc<Mutex<BatchAccumulator>>,
}

impl CognitiveProcessingQueue {
    /// Create new unbounded queue using tokio channels
    pub fn new() -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
            // Default: batch_size=5, max_wait=2000ms
            batch_accumulator: Arc::new(Mutex::new(BatchAccumulator::new(5, 2000))),
        }
    }

    /// Enqueue a task for processing
    pub fn enqueue(&self, task: CognitiveTask) -> Result<(), String> {
        self.sender
            .send(task)
            .map_err(|e| format!("Failed to enqueue task: {}", e))
    }

    /// Enqueue task with automatic batching for CommitteeEvaluation
    ///
    /// SYNC CONTEXT: This function is intentionally non-async to provide
    /// a blocking enqueue operation. Uses `std::sync::Mutex` appropriately.
    pub fn enqueue_with_batching(&self, task: CognitiveTask) -> Result<(), String> {
        // Only batch CommitteeEvaluation tasks
        if matches!(task.task_type, CognitiveTaskType::CommitteeEvaluation) {
            // SYNC LOCK: batch_accumulator uses std::sync::Mutex (not tokio::sync)
            // because this function is NOT async. This is correct usage.
            let mut accumulator = self
                .batch_accumulator
                .lock()
                .map_err(|e| format!("Lock failed: {}", e))?;

            if let Some(batch) = accumulator.add(task) {
                // Convert to BatchProcessing task
                let memory_ids: Vec<String> = batch.into_iter().map(|t| t.memory_id).collect();

                let batch_task = CognitiveTask::new(
                    format!("batch_{}", memory_ids.len()),
                    CognitiveTaskType::BatchProcessing(memory_ids),
                    5, // Medium priority
                );

                self.sender
                    .send(batch_task)
                    .map_err(|e| format!("Failed to enqueue batch: {}", e))?;
            }
            Ok(())
        } else {
            // Non-batchable tasks go directly to queue
            self.enqueue(task)
        }
    }

    /// Flush any pending batches (call before shutdown)
    ///
    /// SYNC CONTEXT: This function is intentionally non-async.
    pub fn flush_batches(&self) -> Result<(), String> {
        // SYNC LOCK: batch_accumulator uses std::sync::Mutex (not tokio::sync)
        // because this function is NOT async. This is correct usage.
        let mut accumulator = self
            .batch_accumulator
            .lock()
            .map_err(|e| format!("Lock failed: {}", e))?;

        if let Some(batch) = accumulator.flush() {
            let memory_ids: Vec<String> = batch.into_iter().map(|t| t.memory_id).collect();

            let batch_task = CognitiveTask::new(
                format!("batch_flush_{}", memory_ids.len()),
                CognitiveTaskType::BatchProcessing(memory_ids),
                5,
            );

            self.sender
                .send(batch_task)
                .map_err(|e| format!("Failed to enqueue flush batch: {}", e))?;
        }
        Ok(())
    }

    /// Dequeue a single task (async, yields if empty)
    pub async fn dequeue(&self) -> Result<CognitiveTask, String> {
        let mut receiver = self.receiver.lock().await;
        receiver
            .recv()
            .await
            .ok_or_else(|| "Channel closed".to_string())
    }

    /// Try to dequeue without blocking
    pub async fn try_dequeue(&self) -> Option<CognitiveTask> {
        let mut receiver = self.receiver.lock().await;
        receiver.try_recv().ok()
    }

    /// Batch dequeue up to `size` tasks
    pub async fn batch_dequeue(&self, size: usize) -> Vec<CognitiveTask> {
        let mut batch = Vec::with_capacity(size);
        let mut receiver = self.receiver.lock().await;
        for _ in 0..size {
            match receiver.try_recv() {
                Ok(task) => batch.push(task),
                Err(_) => break,
            }
        }
        batch
    }

    /// Get approximate queue depth
    pub async fn get_depth_async(&self) -> usize {
        let receiver = self.receiver.lock().await;
        receiver.len()
    }

    /// Get approximate queue depth (sync version, tries lock)
    pub fn get_depth(&self) -> usize {
        if let Ok(receiver) = self.receiver.try_lock() {
            receiver.len()
        } else {
            0
        }
    }
}

impl Default for CognitiveProcessingQueue {
    fn default() -> Self {
        Self::new()
    }
}
