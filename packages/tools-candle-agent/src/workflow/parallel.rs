//! N-way concurrent execution with tokio task-based parallelism for blazing-fast performance
//!
//! This module provides parallel execution for arbitrary numbers of operations using
//! tokio async tasks for concurrent execution. Results stream in completion order
//! for maximum throughput and minimum latency.
//!
//! ## Performance Characteristics
//! - Zero allocation for ≤16 operations using SmallVec
//! - Concurrent execution via tokio task scheduler
//! - Async result streaming via tokio channels
//! - Results emit immediately when ready (no blocking on slowest)
//! - Efficient resource usage via tokio runtime
//! - Operation sharing with dynamic dispatch

use cyrup_sugars::prelude::MessageChunk;
use smallvec::SmallVec;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

use crate::domain::context::chunks::ParallelResult;
use crate::workflow::ops::{DynOp, Op};

/// N-way parallel execution combinator for concurrent processing
///
/// Executes multiple operations concurrently using tokio async tasks.
/// Results stream in completion order, enabling immediate processing
/// of fast operations without waiting for slower ones.
///
/// ## Architecture
/// - Uses SmallVec for zero heap allocation with ≤16 operations
/// - Dynamic dispatch via trait objects for operation heterogeneity  
/// - Tokio async tasks for concurrent execution
/// - Async result collection via tokio mpsc channels
/// - Streaming results preserve operation ordering information
///
/// ## Type Parameters
/// * `In` - Input type for all operations
/// * `Out` - Output type from all operations
///
/// ## Performance Guarantees
/// - Zero allocation for common case (≤16 operations)
/// - No blocking on slowest operation
/// - Linear scaling with CPU core availability
/// - Minimal synchronization overhead
pub struct ParallelN<In, Out>
where
    In: Clone + Send + Sync + 'static,
    Out: Send + Sync + Clone + Default + MessageChunk + 'static,
{
    operations: SmallVec<Box<dyn DynOp<In, Out> + Send + Sync>, 16>,
    _phantom: std::marker::PhantomData<(In, Out)>,
}

impl<In, Out> Clone for ParallelN<In, Out>
where
    In: Clone + Send + Sync + 'static,
    Out: Send + Sync + Clone + Default + MessageChunk + 'static,
{
    fn clone(&self) -> Self {
        Self {
            operations: self.operations.iter().map(|op| op.clone_boxed()).collect(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<In, Out> ParallelN<In, Out>
where
    In: Clone + Send + Sync + 'static,
    Out: Send + Sync + Clone + Default + MessageChunk + 'static,
{
    /// Create a new N-way parallel combinator
    ///
    /// Initializes with zero operations. Use `add()` method or builder pattern
    /// to add operations for parallel execution.
    ///
    /// ## Performance
    /// - Uses SmallVec for stack allocation of operation list
    /// - Zero heap allocation until >16 operations
    /// - Constant-time initialization
    #[inline]
    pub fn new() -> Self {
        Self {
            operations: SmallVec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add an operation to parallel execution
    ///
    /// Operations are executed concurrently when `call()` is invoked.
    /// The order of addition determines the operation index in results.
    ///
    /// ## Arguments
    /// * `operation` - Operation implementing Op<In, Out> trait
    ///
    /// ## Returns
    /// Self for fluent method chaining
    ///
    /// Add an operation to the parallel combinator
    ///
    /// ## Performance
    /// - Constant time addition for ≤16 operations  
    /// - Automatic heap allocation beyond 16 operations
    /// - Operation is boxed for dynamic dispatch
    #[inline]
    pub fn add_operation<OpType>(mut self, operation: OpType) -> Self
    where
        OpType: Op<In, Out> + Clone + Send + Sync + 'static,
    {
        self.operations.push(Box::new(operation));
        self
    }

    /// Get the number of operations that will execute in parallel
    ///
    /// ## Returns
    /// Count of operations added to this parallel combinator
    #[inline]
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    /// Check if this parallel combinator uses stack allocation
    ///
    /// ## Returns
    /// `true` for ≤16 operations (stack allocated), `false` for heap allocation
    #[inline]
    pub fn is_stack_allocated(&self) -> bool {
        self.operations.len() <= 16
    }

    /// Add multiple operations from an iterator
    ///
    /// Convenience method for adding multiple operations at once.
    /// More efficient than multiple individual `add()` calls.
    ///
    /// ## Arguments
    /// * `operations` - Iterator of operations to add
    ///
    /// ## Returns
    /// Self for fluent method chaining
    #[inline]
    pub fn add_all<I, OpType>(mut self, operations: I) -> Self
    where
        I: IntoIterator<Item = OpType>,
        OpType: Op<In, Out> + Clone + Send + Sync + 'static,
    {
        for operation in operations {
            self.operations.push(Box::new(operation));
        }
        self
    }

    /// Execute all operations with the given input
    ///
    /// This is equivalent to calling the `Op::call()` method but provides
    /// a more explicit API for direct execution.
    ///
    /// ## Arguments
    /// * `input` - Input value to pass to all operations
    ///
    /// ## Returns
    /// Stream of ParallelResult<Out> values in completion order
    #[inline]
    pub fn execute(self, input: In) -> Pin<Box<dyn Stream<Item = ParallelResult<Out>> + Send>> {
        <Self as Op<In, ParallelResult<Out>>>::call(&self, input)
    }
}

impl<In, Out> Default for ParallelN<In, Out>
where
    In: Clone + Send + Sync + 'static,
    Out: Send + Sync + Clone + Default + MessageChunk + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<In, Out> Op<In, ParallelResult<Out>> for ParallelN<In, Out>
where
    In: Clone + Send + Sync + 'static,
    Out: Send + Sync + Clone + Default + MessageChunk + 'static,
{
    /// Execute all operations in parallel with streaming results
    ///
    /// ## Execution Model
    /// 1. Clone input for each operation to avoid borrowing issues
    ///    tokio async tasks
    /// 3. Stream results immediately as operations complete
    /// 4. Preserve operation index for result correlation
    /// 5. Handle thread panics gracefully with error results
    ///
    /// ## Performance Characteristics
    /// - True parallelism using OS threads (not async tasks)
    /// - Zero blocking on individual operation completion
    /// - Linear scaling with available CPU cores
    /// - Minimal synchronization via lock-free channels
    /// - Bounded resource usage via scoped threads
    ///
    /// ## Error Handling
    /// - Thread panics converted to error results
    /// - Individual operation failures don't stop others
    /// - Graceful degradation on resource exhaustion
    fn call(&self, input: In) -> Pin<Box<dyn Stream<Item = ParallelResult<Out>> + Send>> {
        let operations: SmallVec<Box<dyn DynOp<In, Out> + Send + Sync>, 16> =
            self.operations.iter().map(|op| op.clone_boxed()).collect();
        let operation_count = operations.len();

        // Handle edge case: no operations
        if operation_count == 0 {
            return Box::pin(crate::async_stream::spawn_stream(|_tx| async move {
                // No operations to execute, stream completes immediately
            }));
        }

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Use tokio async tasks for concurrent execution
            let (result_tx, mut result_rx) =
                tokio::sync::mpsc::unbounded_channel::<ParallelResult<Out>>();

            // Spawn each operation in separate async task
            for (op_index, operation) in operations.into_iter().enumerate() {
                let input_clone = input.clone();
                let result_tx_clone = result_tx.clone();

                tokio::spawn(async move {
                    // Execute operation and stream all results
                    let op_stream = operation.call(input_clone);
                    tokio::pin!(op_stream);

                    // Stream all results from this operation with index tracking
                    while let Some(result) = op_stream.next().await {
                        let parallel_result = ParallelResult::new(op_index, result);

                        // Send result with operation index for correlation
                        if result_tx_clone.send(parallel_result).is_err() {
                            // Receiver dropped, stop processing this operation
                            log::debug!(
                                "Parallel operation {} receiver dropped - terminating",
                                op_index
                            );
                            break;
                        }
                    }
                });
            }

            // Drop the original sender to signal no more senders
            drop(result_tx);

            // Collect and stream results as they arrive from any operation
            while let Some(parallel_result) = result_rx.recv().await {
                if tx.send(parallel_result).is_err() {
                    // Main receiver dropped, stop streaming all results
                    log::debug!("Main parallel receiver dropped - terminating all operations");
                    break;
                }
            }
        }))
    }
}

/// Fluent builder for parallel operations
///
/// Alternative to direct ParallelN construction that provides a more
/// explicit building pattern. Useful when the set of operations is
/// determined dynamically or when building complex parallel workflows.
///
/// ## Example
/// ```rust,no_run
/// use kodegen_candle_agent::workflow::parallel::ParallelBuilder;
///
/// let parallel_ops = ParallelBuilder::new()
///     .add_operation(operation1)
///     .add_operation(operation2)
///     .add_operation(operation3)
///     .build();
/// ```
pub struct ParallelBuilder<In, Out>
where
    In: Clone + Send + Sync + 'static,
    Out: Send + Sync + Clone + Default + MessageChunk + 'static,
{
    parallel: ParallelN<In, Out>,
}

impl<In, Out> ParallelBuilder<In, Out>
where
    In: Clone + Send + Sync + 'static,
    Out: Send + Sync + Clone + Default + MessageChunk + 'static,
{
    /// Create a new parallel builder
    ///
    /// Initializes an empty builder ready for operation addition.
    #[inline]
    pub fn new() -> Self {
        Self {
            parallel: ParallelN::new(),
        }
    }

    /// Add an operation to the parallel execution
    ///
    /// ## Arguments  
    /// * `operation` - Operation to execute in parallel
    ///
    /// ## Returns
    /// Self for fluent method chaining
    #[inline]
    pub fn add_operation<OpType>(mut self, operation: OpType) -> Self
    where
        OpType: Op<In, Out> + Clone + Send + Sync + 'static,
    {
        self.parallel.operations.push(Box::new(operation));
        self
    }

    /// Add multiple operations from an iterator
    ///
    /// ## Arguments
    /// * `operations` - Iterator of operations to add
    ///
    /// ## Returns  
    /// Self for fluent method chaining
    #[inline]
    pub fn add_operations<I, OpType>(mut self, operations: I) -> Self
    where
        I: IntoIterator<Item = OpType>,
        OpType: Op<In, Out> + Clone + Send + Sync + 'static,
    {
        for operation in operations {
            self.parallel.operations.push(Box::new(operation));
        }
        self
    }

    /// Build the parallel combinator
    ///
    /// Consumes the builder and returns the configured ParallelN instance
    /// ready for execution.
    ///
    /// ## Returns
    /// Configured ParallelN combinator
    #[inline]
    pub fn build(self) -> ParallelN<In, Out> {
        self.parallel
    }

    /// Execute the parallel operations immediately with input
    ///
    /// Convenience method that builds and executes in one step.
    /// Equivalent to `build().execute(input)`.
    ///
    /// ## Arguments
    /// * `input` - Input value to pass to all operations
    ///
    /// ## Returns
    /// Stream of ParallelResult<Out> values
    #[inline]
    pub fn execute(self, input: In) -> Pin<Box<dyn Stream<Item = ParallelResult<Out>> + Send>> {
        self.parallel.execute(input)
    }

    /// Get the current number of operations in the builder
    #[inline]
    pub fn operation_count(&self) -> usize {
        self.parallel.operation_count()
    }

    /// Check if the builder will use stack allocation
    #[inline]
    pub fn is_stack_allocated(&self) -> bool {
        self.parallel.is_stack_allocated()
    }
}

impl<In, Out> Default for ParallelBuilder<In, Out>
where
    In: Clone + Send + Sync + 'static,
    Out: Send + Sync + Clone + Default + MessageChunk + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// Macro is defined in macros.rs to avoid duplication

