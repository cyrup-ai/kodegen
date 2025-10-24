//! Compile-time parallel execution tree generation with tuple flattening
//!
//! This module provides macros for generating efficient parallel execution trees
//! at compile time. The macros enable variadic parallel operations with clean
//! tuple output types and optimal performance through compile-time optimization.

/// Internal macro for recursive parallel operation tree construction
///
/// This macro recursively builds parallel execution trees, combining operations
/// in binary pairs and flattening tuple outputs for clean composition. The
/// recursive expansion enables efficient n-way parallelism with compile-time
/// optimization.
///
/// ## Expansion Pattern
/// - Single operation: returns operation unchanged
/// - Two operations: creates Parallel combinator
/// - Multiple operations: recursively builds binary tree structure
/// - Tuple flattening: ensures clean output types without nested tuples
///
/// ## Performance
/// - Compile-time tree generation (zero runtime overhead)
/// - Binary tree structure for optimal parallel execution
/// - Tuple flattening eliminates type complexity
/// - Template specialization for concrete operation types
///
/// # Public macro for ergonomic N-way parallel execution
///
/// Creates a ParallelN combinator that executes all provided operations
/// concurrently with true N-way parallelism. Results stream in completion
/// order with operation index tracking for optimal performance.
///
/// ## Usage
/// ```rust,no_run
/// use kodegen_candle_agent::{parallel, workflow::ops::map};
///
/// let double = map(|x: i32| x * 2);
/// let triple = map(|x: i32| x * 3);
/// let quadruple = map(|x: i32| x * 4);
///
/// // Create N-way parallel execution of all operations
/// let parallel_ops = parallel!(double, triple, quadruple);
/// let result_stream = parallel_ops.call(5);
/// // Result: AsyncStream<ParallelResult<i32>> with results streaming in completion order
/// ```
///
/// ## Performance Benefits
/// - True N-way parallelism (no binary tree nesting overhead)
/// - Zero allocation for ≤16 operations using SmallVec
/// - Results stream immediately when ready (no blocking on slowest)
/// - Compile-time operation counting and optimization
/// - Linear scaling with CPU core availability
///
/// ## Output Structure
/// Results are wrapped in `ParallelResult<T>` which provides:
/// - `operation_index`: Index of the operation that produced the result
/// - `result`: The actual result from the operation
/// - Immediate streaming without waiting for all operations to complete
#[macro_export]
macro_rules! parallel {
    // Base case: single operation (pass through unchanged)
    ($op:expr) => {
        $op
    };

    // N-way parallel case: create ParallelN with all operations
    ($($op:expr),+ $(,)?) => {
        {
            let mut parallel = $crate::workflow::parallel::ParallelN::new();
            $(
                parallel = parallel.add_operation($op);
            )*
            parallel
        }
    };
}

// Internal macro for recursive error-propagating parallel operation tree construction
//
// Similar to parallel_op! but builds trees of TryParallel combinators that
// short-circuit on first error. The recursive expansion creates efficient
// error-propagating parallel execution with clean error handling semantics.
//
// ## Error Propagation Semantics
// - Short-circuit on first error from any parallel branch
// - Consistent error types across all parallel operations
// - Clean error composition without nested Result types
// - Compile-time error type validation and consistency
// try_parallel_internal macro REMOVED - depended on TryOp trait that violated streams-only architecture
// try_parallel macro REMOVED - violated streams-only architecture with Result-wrapped streams

/// Compile-time tuple flattening utilities for clean output types
///
/// These macros provide compile-time tuple manipulation for creating clean
/// output types from variadic parallel operations. The flattening eliminates
/// deeply nested tuple structures for better API ergonomics.
///
/// ## Note on Tuple Flattening
/// Current implementation uses right-associative nesting which is simpler
/// to implement and provides consistent type patterns. Future versions
/// could implement full tuple flattening for completely flat output types
/// if needed for specific use cases.
/// Helper macro to count the number of operations for compile-time optimization
///
/// Counts the number of comma-separated expressions at compile time,
/// enabling size-specific optimizations and type generation.
///
// Re-export main public macro only
pub use parallel;
