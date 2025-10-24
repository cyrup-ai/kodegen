//! Command execution events and context tracking with zero allocation patterns
//!
//! Provides blazing-fast event streaming and execution context management
//! with owned strings allocated once for maximum performance. No Arc usage, no locking.

// Module declarations
pub mod context;
pub mod event_types;
pub mod executor;
pub mod impls;
pub mod stats;

// Re-export all public types for backward compatibility
pub use context::CommandExecutionContext;
pub use event_types::CommandEvent;
pub use executor::StreamingCommandExecutor;
pub use stats::CommandExecutorStats;
