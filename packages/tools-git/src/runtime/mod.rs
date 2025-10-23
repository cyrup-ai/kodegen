//! Runtime module
//!
//! Provides async task execution and streaming primitives.

pub mod async_task;

// Re-export async task types
pub use async_task::{AsyncStream, AsyncTask, EmitterBuilder};
