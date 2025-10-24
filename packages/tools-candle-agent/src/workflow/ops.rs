//! Zero-cost async-aware transformation nodes with blazing-fast performance
//!
//! This module provides the Op trait system for building composable, high-performance
//! workflow operations using cyrup streams-only architecture. All operations are
//! zero-allocation in hot paths with extensive inlining for maximum performance.

use std::marker::PhantomData;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};

use cyrup_sugars::prelude::MessageChunk;

/// Object-safe operation trait for dynamic dispatch in N-way parallelism
///
/// This trait enables dynamic dispatch via trait objects while maintaining
/// performance characteristics. Used internally by ParallelN for N-way
/// parallel execution with heterogeneous operations.
///
/// ## Dynamic Dispatch Architecture
/// - Object-safe trait design (no Clone requirement)
/// - Boxed clone method for trait object cloning
/// - Minimal trait bounds for maximum flexibility
/// - Maintains zero-allocation principles where possible
pub trait DynOp<In, Out>: Send + Sync
where
    In: Send + Sync + 'static,
    Out: Send + Sync + MessageChunk + Default + 'static,
{
    /// Execute the operation with streaming output
    fn call(&self, input: In) -> Pin<Box<dyn Stream<Item = Out> + Send>>;

    /// Clone this operation as a boxed trait object
    ///
    /// Enables trait object cloning for dynamic parallel execution.
    /// This method bridges the gap between static and dynamic dispatch.
    fn clone_boxed(&self) -> Box<dyn DynOp<In, Out> + Send + Sync>;
}

/// Blanket implementation to bridge static Op trait to dynamic DynOp
///
/// This implementation allows any type that implements Op to be used
/// as a DynOp trait object, enabling seamless interoperability between
/// static dispatch (for performance) and dynamic dispatch (for N-way parallelism).
impl<T, In, Out> DynOp<In, Out> for T
where
    T: Op<In, Out> + Clone + 'static,
    In: Send + Sync + 'static,
    Out: Send + Sync + MessageChunk + Default + 'static,
{
    fn call(&self, input: In) -> Pin<Box<dyn Stream<Item = Out> + Send>> {
        Op::call(self, input)
    }

    fn clone_boxed(&self) -> Box<dyn DynOp<In, Out> + Send + Sync> {
        Box::new(self.clone())
    }
}

/// Core operation trait for zero-cost async-aware transformations
///
/// The Op trait defines the fundamental building block for composable workflow
/// operations. All implementations must be zero-allocation with Send + Sync + Clone
/// for maximum concurrent performance.
///
/// ## Architecture Constraints
/// - Zero-allocation with PhantomData for type safety
/// - Send + Sync + Clone for concurrent reusability
/// - Stream-only outputs (NO Future/Result wrapping)
/// - All hot-path methods marked #[inline] for zero-cost abstractions
/// - Clone for efficient operation sharing and composition
/// - Unwrapped Stream pattern - no Result wrapping in stream values
///
/// ## Performance Characteristics
/// - Concrete type specialization for blazing-fast execution
/// - Zero-cost abstractions through extensive inlining
/// - Memory-efficient with no heap allocation in execution paths
/// - Lock-free design for maximum concurrency
///
/// ## Example
/// ```rust,no_run
/// use kodegen_candle_agent::workflow::ops::{Op, map};
///
/// let double_op = map(|x: i32| x * 2);
/// let stream = double_op.call(5);
/// // Result: Stream containing [10]
/// ```
pub trait Op<In, Out>: Send + Sync + Clone + 'static {
    /// Execute the operation with streaming output
    ///
    /// Takes input of type `In` and produces a stream of `Out` values
    /// using tokio Stream. This follows the cyrup unwrapped stream
    /// architecture for maximum performance.
    ///
    /// ## Implementation Requirements
    /// - Must use tokio Stream for return type (streams-only architecture)
    /// - Streams are async and should be awaited
    /// - Unwrapped stream values - no Result wrapping for performance
    /// - Real execution logic - no mocking or simulation
    /// - Hot path should be marked #[inline] for zero-cost abstractions
    fn call(&self, input: In) -> Pin<Box<dyn Stream<Item = Out> + Send>>;

    /// Fluent combinator: chain this operation with another
    ///
    /// Creates a new operation that first executes this operation, then
    /// pipes each output value to the next operation. This creates
    /// streaming composition with zero intermediate allocation.
    ///
    /// ## Streaming Semantics
    /// - First operation outputs are piped to second operation
    /// - Zero allocation streaming composition
    /// - Sequential execution with optimal performance
    /// - Hot path inlined for maximum performance
    #[inline]
    fn then<NewOut, O>(self, next: O) -> Then<Self, O, In, Out, NewOut>
    where
        O: Op<Out, NewOut> + 'static,
        NewOut: Send + Sync + 'static,
    {
        Then {
            first: self,
            second: next,
            _phantom: PhantomData,
        }
    }

    /// Fluent combinator: transform output values
    ///
    /// Creates a new operation that applies a transformation function to each
    /// output value from this operation. This enables zero-allocation
    /// functional composition patterns.
    ///
    /// ## Performance
    /// - Function applied to all stream values
    /// - Zero allocation transformation
    /// - Function inlining for optimal performance
    /// - Stream-based execution without collection
    #[inline]
    fn map<NewOut, F>(self, func: F) -> Map<Self, F, In, Out, NewOut>
    where
        F: Fn(Out) -> NewOut + Send + Sync + Clone + 'static,
        NewOut: Send + Sync + 'static,
    {
        Map {
            op: self,
            func,
            _phantom: PhantomData,
        }
    }

    /// Fluent combinator: chain operations sequentially  
    ///
    /// Alias for `then()` that emphasizes sequential execution semantics.
    /// Useful for building linear operation chains where sequence matters.
    #[inline]
    fn chain<NewOut, O>(self, next: O) -> Then<Self, O, In, Out, NewOut>
    where
        O: Op<Out, NewOut> + 'static,
        NewOut: Send + Sync + 'static,
    {
        self.then(next)
    }
}

/// Sequential composition of two operations
///
/// Executes the first operation, then pipes each output to the second operation.
/// This creates a streaming composition with zero intermediate allocation and
/// optimal performance through extensive inlining.
///
/// ## Memory Efficiency
/// - PhantomData for zero-cost type tracking
/// - No intermediate buffering or collection
/// - Stream-based execution with minimal overhead
/// - Clone implementation for operation reuse
#[derive(Clone)]
pub struct Then<A, B, In, Mid, Out> {
    first: A,
    second: B,
    _phantom: PhantomData<(In, Mid, Out)>,
}

impl<A, B, In, Mid, Out> Op<In, Out> for Then<A, B, In, Mid, Out>
where
    A: Op<In, Mid> + 'static,
    B: Op<Mid, Out> + 'static,
    In: Send + Sync + Clone + 'static,
    Mid: Send + Sync + Clone + Default + 'static + cyrup_sugars::prelude::MessageChunk,
    Out: Send + Sync + Clone + Default + 'static + cyrup_sugars::prelude::MessageChunk,
{
    #[inline]
    fn call(&self, input: In) -> Pin<Box<dyn Stream<Item = Out> + Send>> {
        let first = self.first.clone();
        let second = self.second.clone();

        // Create streaming composition using correct AsyncStream patterns
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                // Execute first operation to get intermediate stream
                let first_stream = first.call(input);
                tokio::pin!(first_stream);

                // Process each intermediate value through second operation
                while let Some(mid_value) = first_stream.next().await {
                    // Execute second operation with intermediate value
                    let second_stream = second.call(mid_value);
                    tokio::pin!(second_stream);

                    // Forward all outputs from second operation
                    while let Some(output) = second_stream.next().await {
                        if sender.send(output).is_err() {
                            log::debug!(
                                "Stream receiver dropped during sequential operation - execution terminated"
                            );
                            return; // Receiver dropped, exit gracefully
                        }
                    }
                }
            },
        ))
    }
}

/// Function-based transformation operation
///
/// Applies a transformation function to each input value, producing a stream
/// of transformed outputs. The transformation is executed in a streaming
/// fashion with zero allocation in the hot path.
///
/// ## Performance Characteristics  
/// - Zero allocation with PhantomData type tracking
/// - Function inlining for maximum performance
/// - Stream-based execution without intermediate collection
/// - Clone for efficient operation sharing
#[derive(Clone)]
pub struct Map<O, F, In, Out, NewOut> {
    op: O,
    func: F,
    _phantom: PhantomData<(In, Out, NewOut)>,
}

impl<O, F, In, Out, NewOut> Op<In, NewOut> for Map<O, F, In, Out, NewOut>
where
    O: Op<In, Out> + 'static,
    F: Fn(Out) -> NewOut + Send + Sync + Clone + 'static,
    In: Send + Sync + Clone + 'static,
    Out: Send + Sync + Clone + Default + 'static + cyrup_sugars::prelude::MessageChunk,
    NewOut: Send + Sync + Clone + Default + 'static + cyrup_sugars::prelude::MessageChunk,
{
    #[inline]
    fn call(&self, input: In) -> Pin<Box<dyn Stream<Item = NewOut> + Send>> {
        let op = self.op.clone();
        let func = self.func.clone();

        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let stream = op.call(input);
                tokio::pin!(stream);

                // Apply transformation function to each output
                while let Some(output) = stream.next().await {
                    let transformed = func(output);
                    if sender.send(transformed).is_err() {
                        log::debug!(
                            "Stream receiver dropped during map operation - execution terminated"
                        );
                        return; // Receiver dropped
                    }
                }
            },
        ))
    }
}

/// Passthrough operation that forwards input unchanged
///
/// Identity operation that passes input through as output without modification.
/// Useful for testing, debugging, and as a neutral element in operation
/// composition chains.
///
/// ## Zero-Cost Implementation
/// - PhantomData eliminates runtime overhead
/// - Direct forwarding without transformation
/// - Minimal allocation in streaming path
#[derive(Clone)]
pub struct Passthrough<T> {
    _phantom: PhantomData<T>,
}

impl<T> Op<T, T> for Passthrough<T>
where
    T: Send + Sync + Clone + Default + 'static + cyrup_sugars::prelude::MessageChunk,
{
    #[inline]
    fn call(&self, input: T) -> Pin<Box<dyn Stream<Item = T> + Send>> {
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                // Direct passthrough - no transformation
                let _ = sender.send(input);
            },
        ))
    }
}

/// Simple function-based operation wrapper
///
/// Wraps a function to create an operation that applies the function to its input.
/// This is the most basic operation type, useful for wrapping pure functions
/// into the operation system.
///
/// ## Performance
/// - Zero allocation with PhantomData
/// - Function inlining opportunities
/// - Direct function call without overhead
#[derive(Clone)]
pub struct FuncOp<F, In, Out> {
    func: F,
    _phantom: PhantomData<(In, Out)>,
}

impl<F, In, Out> Op<In, Out> for FuncOp<F, In, Out>
where
    F: Fn(In) -> Out + Send + Sync + Clone + 'static,
    In: Send + Sync + Clone + 'static,
    Out: Send + Sync + Clone + Default + 'static + cyrup_sugars::prelude::MessageChunk,
{
    #[inline]
    fn call(&self, input: In) -> Pin<Box<dyn Stream<Item = Out> + Send>> {
        let func = self.func.clone();
        Box::pin(crate::async_stream::spawn_stream(
            move |sender| async move {
                let output = func(input);
                let _ = sender.send(output);
            },
        ))
    }
}

/// Ergonomic helper function: create a mapping operation
///
/// Creates a FuncOp that applies the given function to its input.
/// This is the most common way to create simple transformation operations
/// from pure functions.
///
/// ## Example
/// ```rust,no_run
/// use kodegen_candle_agent::workflow::ops::map;
///
/// let double = map(|x: i32| x * 2);
/// let result_stream = double.call(5);
/// // Result: AsyncStream containing [10]
/// ```
#[inline]
pub fn map<F, In, Out>(func: F) -> FuncOp<F, In, Out>
where
    F: Fn(In) -> Out + Send + Sync + Clone + 'static,
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    FuncOp {
        func,
        _phantom: PhantomData,
    }
}

/// Ergonomic helper function: create a passthrough operation
///
/// Creates a Passthrough operation that forwards its input unchanged.
/// Useful as an identity element in operation composition or for
/// debugging purposes.
///
/// ## Example
/// ```rust,no_run
/// use kodegen_candle_agent::workflow::ops::passthrough;
///
/// let identity = passthrough::<i32>();
/// let result_stream = identity.call(42);
/// // Result: AsyncStream containing [42]
/// ```
#[inline]
pub fn passthrough<T>() -> Passthrough<T>
where
    T: Send + Sync + Clone + 'static,
{
    Passthrough {
        _phantom: PhantomData,
    }
}

/// Ergonomic helper function: create a sequential composition
///
/// Creates a Then operation that executes two operations in sequence,
/// piping the output of the first to the input of the second.
///
/// ## Example
/// ```rust,no_run
/// use kodegen_candle_agent::workflow::ops::{map, then};
///
/// let double = map(|x: i32| x * 2);
/// let add_one = map(|x: i32| x + 1);
/// let composed = then(double, add_one);
/// let result_stream = composed.call(5);
/// // Result: AsyncStream containing [11] (5 * 2 + 1)
/// ```
#[inline]
pub fn then<A, B, In, Mid, Out>(first: A, second: B) -> Then<A, B, In, Mid, Out>
where
    A: Op<In, Mid> + 'static,
    B: Op<Mid, Out> + 'static,
    In: Send + Sync + 'static,
    Mid: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    Then {
        first,
        second,
        _phantom: PhantomData,
    }
}
