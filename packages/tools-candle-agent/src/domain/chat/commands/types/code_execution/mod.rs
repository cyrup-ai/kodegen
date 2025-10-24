//! Code execution tool definitions with zero allocation patterns
//!
//! Provides comprehensive tool type definitions and structures for secure code execution
//! across multiple programming languages with owned strings allocated once for maximum
//! performance. No Arc usage, no locking.

// Module declarations
pub mod errors;
pub mod language;
pub mod limits;
pub mod request;
pub mod response;
pub mod tool;
pub mod validation;

// Re-export all public types for backward compatibility
pub use errors::ValidationError;
pub use language::CodeLanguage;
pub use limits::ResourceLimits;
pub use request::CodeExecutionRequest;
pub use response::{CodeExecutionResult, ExecutionError, ExecutionStatus, OutputFormat};
pub use tool::CodeExecutionTool;
pub use validation::ValidationConfig;
