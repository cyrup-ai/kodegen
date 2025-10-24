//! Type definitions for memory coordinator

/// Strategy for handling memories with pending cognitive evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LazyEvalStrategy {
    /// Wait for background processing to complete (polls with timeout)
    WaitForCompletion,
    /// Return immediately with partial data (non-blocking, default)
    #[default]
    ReturnPartial,
    /// Trigger immediate evaluation and wait (bypasses queue)
    TriggerAndWait,
}
