//! Core executor trait for domain command execution
//!
//! Defines the contract that all command executors must implement.
//! Uses zero-allocation patterns and lock-free data structures for maximum performance.

use std::pin::Pin;
use tokio_stream::Stream;

use super::{
    CommandExecutionContext, CommandExecutionResult, CommandInfo, ImmutableChatCommand,
    ValidationResult,
};

/// Domain command executor trait for consistent execution interface
/// Uses zero-allocation patterns and lock-free data structures
pub trait DomainCommandExecutor: Send + Sync + 'static {
    /// Execute command and return stream of results - zero allocation where possible
    fn execute(
        &self,
        context: &CommandExecutionContext,
    ) -> Pin<Box<dyn Stream<Item = CommandExecutionResult> + Send>>;

    /// Get command metadata - returns borrowed data to avoid allocation
    fn get_info(&self) -> &CommandInfo;

    /// Validate command parameters - zero allocation validation
    ///
    /// # Errors
    ///
    /// Returns `CandleCommandError` if command parameters are invalid
    fn validate_parameters(&self, command: &ImmutableChatCommand) -> ValidationResult;

    /// Get command name as static string slice for zero allocation
    fn name(&self) -> &'static str;

    /// Get estimated execution time in milliseconds for scheduling
    fn estimated_duration_ms(&self) -> u64 {
        1000 // Default 1 second
    }

    /// Get memory requirements in bytes for resource planning
    fn memory_requirement_bytes(&self) -> u64 {
        1024 * 1024 // Default 1MB
    }

    /// Check if command can be parallelized with other commands
    fn is_parallelizable(&self) -> bool {
        true // Default to parallelizable
    }
}
