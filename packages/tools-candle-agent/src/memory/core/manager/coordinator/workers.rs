//! Cognitive worker management

use crate::memory::core::cognitive_queue::CognitiveTask;
use crate::memory::utils::Result;

use super::lifecycle::MemoryCoordinator;

impl MemoryCoordinator {
    /// Spawn background cognitive workers
    ///
    /// Returns the number of workers successfully spawned
    ///
    /// # Errors
    ///
    /// Returns `Error::Internal` if unable to store worker handles due to lock poisoning
    pub fn spawn_cognitive_workers(&self, worker_count: usize) -> Result<usize> {
        use crate::memory::core::cognitive_worker::CognitiveWorker;

        for i in 0..worker_count {
            let queue = self.cognitive_queue.clone();
            let manager = self.surreal_manager.clone();
            let evaluator = self.committee_evaluator.clone();

            let worker = CognitiveWorker::new(queue, manager, evaluator);

            // Spawn on main tokio runtime (workers are Send now)
            tokio::spawn(async move {
                log::info!("Cognitive worker {} started", i);
                worker.run().await;
                log::info!("Cognitive worker {} stopped", i);
            });
        }

        log::info!("Spawned {} cognitive worker tasks", worker_count);
        Ok(worker_count)
    }

    /// Enqueue a cognitive task for background processing
    ///
    /// Returns immediately after queuing, processing happens asynchronously
    ///
    /// # Errors
    ///
    /// Returns `Error::Internal` if unable to enqueue due to channel closure
    pub fn enqueue_cognitive_task(&self, task: CognitiveTask) -> Result<()> {
        self.cognitive_queue
            .enqueue(task)
            .map_err(crate::memory::utils::Error::Internal)
    }
}
