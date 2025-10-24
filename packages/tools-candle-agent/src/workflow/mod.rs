//! Candle workflow execution system - streams-only architecture
//!
//! This module provides a complete workflow execution system built on cyrup
//! streams-only architecture with candle prefixes. All operations use unwrapped
//! AsyncStream<Out> without Future/Result wrapping in execution paths.
//!
//! ## Core Components
//! - **core**: CandleWorkflowStep trait and CandleExecutableWorkflow struct
//! - **ops**: Zero-cost operation combinators and transformations
//! - **parallel**: Thread-based parallel execution combinators  
//! - **macros**: Compile-time variadic parallel execution macros
//!
//! ## Architecture Principles
//! - Zero-allocation with PhantomData for type safety
//! - Streams-only execution (NO Result<T,E> wrapping inside streams)
//! - impl Trait patterns only (NO Arc<dyn> or Box<dyn> trait objects)
//! - Thread-based concurrency (no tokio dependency)
//! - Extensive inlining for blazing-fast performance
//! - Lock-free design for maximum throughput

pub mod core;
pub mod macros;
pub mod ops;
pub mod parallel;

// Re-export candle core types for ergonomic imports
pub use core::{CandleExecutableWorkflow, CandleWorkflowStep, candle_workflow};

// Re-export main public macro and types
pub use macros::parallel;
pub use ops::{DynOp, Op, map, passthrough, then};
pub use parallel::{ParallelBuilder, ParallelN};
