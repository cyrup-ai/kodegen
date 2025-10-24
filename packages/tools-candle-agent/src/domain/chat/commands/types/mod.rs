//! Domain command types module - zero allocation, lock-free, blazing-fast implementation
//!
//! Provides ultra-performant command type system with focused, single-responsibility
//! submodules using owned strings allocated once for maximum performance.

// Re-export all submodule types for convenient access - zero allocation re-exports
pub use self::{
    actions::*,
    code_execution::*,
    commands::*,
    errors::*,
    events::*,
    executor_defs::*,
    executor_enum::DomainCommandExecutorEnum,
    // Executor re-exports
    executor_trait::DomainCommandExecutor,
    metadata::ResourceUsage,
    metadata::*,
    parameters::*,
};

// Type aliases for backwards compatibility and consistent naming
pub type CommandContext = CommandExecutionContext;
pub type CommandOutput = CommandOutputData;

/// Command output data with execution metadata
#[derive(Debug, Clone)]
pub struct CommandOutputData {
    /// Unique execution identifier
    pub execution_id: u64,
    /// Output content
    pub content: String,
    /// Output type/format
    pub output_type: OutputType,
    /// Execution time in microseconds
    pub execution_time: u64,
    /// Resource usage statistics
    pub resource_usage: Option<ResourceUsage>,
    /// Timestamp in nanoseconds since epoch
    pub timestamp_nanos: u64,
    /// Whether this is the final output
    pub is_final: bool,
    /// Success status
    pub success: bool,
    /// Optional message for context
    pub message: String,
    /// Optional structured data
    pub data: Option<serde_json::Value>,
}

// Submodules with clear separation of concerns and single responsibilities
pub mod actions; // Action type definitions for command variants
pub mod code_execution; // Code execution tool definitions and structures

// Command modules - decomposed from monolithic commands.rs
pub mod command_core; // Core ImmutableChatCommand enum definition
pub mod command_enums; // Settings and output type enumerations
pub mod command_introspection; // Command metadata and introspection methods
pub mod command_results; // Command execution result types
pub mod command_validation; // Command validation logic

pub mod commands; // Command type aggregator with 5 focused submodules
pub mod errors; // Command errors and result types
pub mod events; // Command execution events and context tracking
pub mod metadata; // Command metadata and resource tracking
pub mod parameters; // Parameter definitions and validation

// Executor modules - zero allocation command execution infrastructure
pub mod action_executor_impls;
pub mod core_executor_impls; // Core command implementations
pub mod data_executor_impls; // Data operation implementations
pub mod executor_defs; // Executor struct definitions
pub mod executor_enum; // Zero-allocation enum dispatch
pub mod executor_trait; // DomainCommandExecutor trait definition
pub mod workflow_executor_impls; // Workflow command implementations // Action command implementations
