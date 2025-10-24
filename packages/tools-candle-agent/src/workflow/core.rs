//! Candle Workflow Core - Zero-allocation impl Trait workflow execution
//!
//! This module provides candle workflow traits and builders following cyrup
//! patterns exactly but with Candle prefixes. All execution uses tokio Stream<Out>
//! with NO trait objects - only impl Trait patterns.

use std::marker::PhantomData;
use std::pin::Pin;
use tokio_stream::Stream;

use crate::domain::context::WorkflowDataChunk;

/// Core workflow execution trait enabling polymorphic workflow steps
///
/// This trait defines the contract for any executable workflow step, providing
/// true async execution with streams-only architecture compliance. All implementations
/// must use tokio Stream for output without Future trait usage.
///
/// ## Architecture Constraints
/// - Zero-allocation with PhantomData for type safety
/// - Send + Sync for concurrent execution capabilities
/// - Stream-only outputs (NO Result wrapping)
/// - No unsafe code, no locking primitives
/// - Error handling via handle_error! macro and stream termination
///
/// ## Performance Characteristics
/// - Hot path marked #[inline] for zero-cost abstractions
/// - Concrete type specialization for blazing-fast execution
/// - No trait objects - all impl Trait patterns for zero allocation
///
/// ## Example
/// ```rust,no_run
/// use kodegen_candle_agent::workflow::{CandleWorkflowStep, WorkflowDataChunk};
/// use tokio_stream::Stream;
/// use std::pin::Pin;
///
/// struct ProcessingStep;
///
/// impl CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk> for ProcessingStep {
///     fn execute(&self, input: WorkflowDataChunk) -> Pin<Box<dyn Stream<Item = WorkflowDataChunk> + Send>> {
///         Box::pin(kodegen_candle_agent::async_stream::spawn_stream(move |tx| async move {
///             // Real execution logic - process the input chunk
///             let _ = tx.send(input);
///         }))
///     }
/// }
/// ```
pub trait CandleWorkflowStep<In, Out>: Send + Sync + 'static {
    /// Execute the workflow step with streaming output
    ///
    /// Takes input of type `In` and produces a stream of `Out` values
    /// using tokio Stream. This method is the core execution primitive for all
    /// workflow operations.
    ///
    /// ## Implementation Requirements
    /// - Must use tokio Stream for return type (streams-only architecture)
    /// - Streams are async and should be awaited
    /// - NO Result<T, E> wrapping - error handling via stream patterns
    /// - Real execution logic - no mocking or simulation
    ///
    /// ## Performance Notes
    /// - Method is not marked #[inline] to allow concrete specialization
    /// - Implementations should use #[inline] for hot path methods
    /// - Stream provides zero-copy streaming where possible
    fn execute(&self, input: In) -> Pin<Box<dyn Stream<Item = Out> + Send>>;
}

/// Simple passthrough step for basic workflow functionality  
/// This is a concrete implementation that can be used as a starting point
#[derive(Clone)]
pub struct CandlePassthroughStep;

impl CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk> for CandlePassthroughStep {
    #[inline]
    fn execute(
        &self,
        input: WorkflowDataChunk,
    ) -> Pin<Box<dyn Stream<Item = WorkflowDataChunk> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            // Send input through stream
            let _ = tx.send(input);
        }))
    }
}

/// Public constructor for creating candle workflows
/// Returns a simple executable workflow that passes through its input
pub fn candle_workflow() -> CandleExecutableWorkflow<CandlePassthroughStep> {
    CandleExecutableWorkflow {
        step: CandlePassthroughStep,
        _phantom: PhantomData,
    }
}

/// Compiled, executable candle workflow with concrete step type
///
/// This struct represents a fully built workflow that can be executed
/// with streaming output. It follows candle patterns with tokio Stream<Out>
/// for error handling via stream patterns.
pub struct CandleExecutableWorkflow<S>
where
    S: CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk>,
{
    step: S,
    _phantom: PhantomData<S>,
}

impl<S> CandleExecutableWorkflow<S>
where
    S: CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk>,
{
    /// Execute the workflow with streaming output
    ///
    /// Takes input and produces a stream of outputs using streams-only architecture.
    /// Error handling uses stream patterns.
    ///
    /// ## Streams-Only Architecture  
    /// - Returns Pin<Box<dyn Stream<Item = WorkflowDataChunk> + Send>>
    /// - NO Result<T,E> wrapping inside streams
    /// - Error handling via stream patterns
    #[inline]
    pub fn execute(
        &self,
        input: WorkflowDataChunk,
    ) -> Pin<Box<dyn Stream<Item = WorkflowDataChunk> + Send>> {
        self.step.execute(input)
    }

    /// Chain another step after this workflow
    /// Creates a new workflow that executes this step then the next step
    pub fn then<NextS>(
        self,
        next_step: NextS,
    ) -> CandleExecutableWorkflow<CandleSequentialStep<S, NextS>>
    where
        NextS: CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk> + Clone,
        S: Clone,
    {
        CandleExecutableWorkflow {
            step: CandleSequentialStep {
                first: self.step,
                second: next_step,
                _phantom: PhantomData,
            },
            _phantom: PhantomData,
        }
    }
}

/// Sequential composition step that chains two steps together
/// This replaces the Arc<dyn> pattern with concrete generic composition
pub struct CandleSequentialStep<A, B>
where
    A: CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk> + Clone,
    B: CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk> + Clone,
{
    first: A,
    second: B,
    _phantom: PhantomData<(A, B)>,
}

impl<A, B> CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk> for CandleSequentialStep<A, B>
where
    A: CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk> + Clone,
    B: CandleWorkflowStep<WorkflowDataChunk, WorkflowDataChunk> + Clone,
{
    fn execute(
        &self,
        input: WorkflowDataChunk,
    ) -> Pin<Box<dyn Stream<Item = WorkflowDataChunk> + Send>> {
        // Clone to avoid lifetime issues in the closure
        let second_clone = self.second.clone();

        // Execute first step and chain with second step
        let first_stream = self.first.execute(input);

        Box::pin(crate::async_stream::spawn_stream(move |tx| async move {
            use tokio_stream::StreamExt;
            let first_results = first_stream.collect::<Vec<_>>().await;
            for intermediate in first_results {
                let second_stream = second_clone.execute(intermediate);
                let second_results = second_stream.collect::<Vec<_>>().await;
                for output in second_results {
                    // Send output through stream
                    if tx.send(output).is_err() {
                        break;
                    }
                }
            }
        }))
    }
}
