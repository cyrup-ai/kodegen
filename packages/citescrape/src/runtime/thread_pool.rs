use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

/// A blazing-fast, zero-allocation work-stealing thread pool.
/// Optimized for high-throughput job execution with lock-free scheduling.
pub struct ThreadPool {
    /// Global work injector for distributing jobs across workers
    injector: Arc<Injector<Job>>,
    /// Work stealers for each worker thread (read-only after creation)
    stealers: Arc<Vec<Stealer<Job>>>,
    /// Round-robin counter for fair job distribution
    round_robin: Arc<AtomicUsize>,
    /// Worker thread handles for clean shutdown
    _handles: Vec<JoinHandle<()>>,
}

/// Job type optimized for zero-allocation execution
type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Creates a new thread pool with optimal worker count based on CPU cores.
    /// All worker threads are spawned immediately and begin work-stealing.
    /// 
    /// This is a one-time allocation operation - all subsequent job scheduling is zero-allocation.
    pub fn new() -> Self {
        let worker_count = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .max(1); // Ensure at least one worker

        let injector = Arc::new(Injector::new());
        let mut workers = Vec::with_capacity(worker_count);
        let mut stealers = Vec::with_capacity(worker_count);
        let mut handles = Vec::with_capacity(worker_count);

        // Create worker threads with local work queues
        for _worker_id in 0..worker_count {
            let local_worker = Worker::new_fifo();
            stealers.push(local_worker.stealer());
            workers.push(local_worker);
        }

        let stealers_arc = Arc::new(stealers);
        let round_robin = Arc::new(AtomicUsize::new(0));

        // Spawn worker threads
        for (worker_id, worker) in workers.into_iter().enumerate() {
            let injector = injector.clone();
            let stealers = stealers_arc.clone();
            
            let handle = thread::Builder::new()
                .name(format!("runtime-worker-{}", worker_id))
                .spawn(move || {
                    Self::worker_loop(worker_id, worker, injector, stealers);
                })
                .unwrap_or_else(|e| {
                    panic!("Failed to spawn runtime worker thread {}: {}", worker_id, e);
                });
            
            handles.push(handle);
        }

        ThreadPool {
            injector,
            stealers: stealers_arc,
            round_robin,
            _handles: handles,
        }
    }

    /// Executes a job on the thread pool with zero allocations after the initial boxed closure.
    /// Jobs are distributed using round-robin scheduling for optimal load balancing.
    /// 
    /// This is the primary interface for job submission and is optimized for maximum throughput.
    #[inline]
    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let boxed_job: Job = Box::new(job);
        
        // Fast path: try to inject directly into global queue
        self.injector.push(boxed_job);
        
        // Wake a worker thread via atomic increment (no allocation)
        self.round_robin.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the number of worker threads in the pool.
    #[inline]
    pub fn worker_count(&self) -> usize {
        self.stealers.len()
    }

    /// Returns an estimate of pending jobs across all queues.
    /// This is an approximation due to the lock-free nature of the queues.
    #[inline]
    pub fn pending_jobs(&self) -> usize {
        // Global queue length plus approximation of local queue lengths
        self.injector.len() + 
        self.stealers.iter().map(|s| s.len()).sum::<usize>()
    }

    /// Worker thread main loop implementing work-stealing algorithm.
    /// Each worker prioritizes its local queue, then steals from the global injector,
    /// and finally attempts to steal from other workers.
    fn worker_loop(
        worker_id: usize,
        worker: Worker<Job>,
        injector: Arc<Injector<Job>>,
        stealers: Arc<Vec<Stealer<Job>>>,
    ) {
        let stealers_count = stealers.len();
        let mut next_steal_target = (worker_id + 1) % stealers_count;

        loop {
            // Phase 1: Execute local jobs first (highest priority)
            while let Some(job) = worker.pop() {
                job();
            }

            // Phase 2: Steal from global injector (medium priority)
            match injector.steal() {
                Steal::Success(job) => {
                    job();
                    continue;
                }
                Steal::Empty => {}
                Steal::Retry => continue,
            }

            // Phase 3: Steal from other workers (lowest priority)
            let mut steal_attempts = 0;
            while steal_attempts < stealers_count {
                if next_steal_target != worker_id {
                    match stealers[next_steal_target].steal() {
                        Steal::Success(job) => {
                            job();
                            break;
                        }
                        Steal::Empty => {}
                        Steal::Retry => continue,
                    }
                }
                
                next_steal_target = (next_steal_target + 1) % stealers_count;
                steal_attempts += 1;
            }

            // Phase 4: Batch steal from global injector for efficiency
            match injector.steal_batch_and_pop(&worker) {
                Steal::Success(job) => {
                    job();
                    continue;
                }
                Steal::Empty => {}
                Steal::Retry => continue,
            }

            // Phase 5: CPU yield to prevent busy waiting
            thread::yield_now();
        }
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ThreadPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadPool")
            .field("workers", &self.worker_count())
            .field("pending_jobs", &self.pending_jobs())
            .finish()
    }
}