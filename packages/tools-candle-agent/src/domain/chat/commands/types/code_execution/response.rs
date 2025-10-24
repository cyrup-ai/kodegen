//! Code execution response types

use serde::{Deserialize, Serialize};

use super::super::metadata::ResourceUsage;

/// Expected output format for code execution results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Plain text output
    Text,
    /// JSON structured output
    Json,
    /// CSV tabular output
    Csv,
    /// Binary output (base64 encoded)
    Binary,
    /// Image output (base64 encoded)
    Image,
}

impl OutputFormat {
    /// Get format name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Binary => "binary",
            Self::Image => "image",
        }
    }
}

/// Code execution result with comprehensive information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionResult {
    /// Exit code from the executed process
    pub exit_code: i32,
    /// Standard output from execution (owned string allocated once)
    pub stdout: String,
    /// Standard error from execution (owned string allocated once)
    pub stderr: String,
    /// Execution duration in microseconds
    pub duration_us: u64,
    /// Resource usage during execution
    pub resource_usage: ResourceUsage,
    /// Execution status
    pub status: ExecutionStatus,
    /// Error information if execution failed
    pub error: Option<ExecutionError>,
    /// Validation warnings generated during execution
    pub warnings: Vec<String>,
    /// Output format of the result
    pub output_format: OutputFormat,
    /// Additional metadata about execution
    pub metadata: std::collections::HashMap<String, String>,
}

impl CodeExecutionResult {
    /// Create a successful execution result
    #[inline]
    pub fn success(
        exit_code: i32,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
        duration_us: u64,
        resource_usage: ResourceUsage,
    ) -> Self {
        Self {
            exit_code,
            stdout: stdout.into(),
            stderr: stderr.into(),
            duration_us,
            resource_usage,
            status: ExecutionStatus::Success,
            error: None,
            warnings: Vec::new(),
            output_format: OutputFormat::Text,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a failed execution result
    #[inline]
    pub fn failure(
        exit_code: i32,
        stderr: impl Into<String>,
        duration_us: u64,
        error: ExecutionError,
    ) -> Self {
        Self {
            exit_code,
            stdout: String::new(),
            stderr: stderr.into(),
            duration_us,
            resource_usage: ResourceUsage::default(),
            status: ExecutionStatus::Failed,
            error: Some(error),
            warnings: Vec::new(),
            output_format: OutputFormat::Text,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a timeout execution result
    #[inline]
    #[must_use]
    pub fn timeout(duration_us: u64) -> Self {
        Self {
            exit_code: -1,
            stdout: String::new(),
            stderr: "Execution timed out".to_string(),
            duration_us,
            resource_usage: ResourceUsage::default(),
            status: ExecutionStatus::Timeout,
            error: Some(ExecutionError::Timeout),
            warnings: Vec::new(),
            output_format: OutputFormat::Text,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Check if execution was successful
    #[inline]
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Success) && self.exit_code == 0
    }

    /// Check if execution failed
    #[inline]
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self.status, ExecutionStatus::Failed)
    }

    /// Check if execution timed out
    #[inline]
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        matches!(self.status, ExecutionStatus::Timeout)
    }

    /// Get execution duration as human readable string
    #[allow(clippy::cast_precision_loss)] // Acceptable for display formatting
    #[inline]
    #[must_use]
    pub fn duration_human(&self) -> String {
        let duration_us = self.duration_us;

        if duration_us < 1000 {
            format!("{duration_us}μs")
        } else if duration_us < 1_000_000 {
            format!("{:.1}ms", duration_us as f64 / 1000.0)
        } else if duration_us < 60_000_000 {
            format!("{:.2}s", duration_us as f64 / 1_000_000.0)
        } else {
            let minutes = duration_us / 60_000_000;
            let seconds = (duration_us % 60_000_000) / 1_000_000;
            format!("{minutes}m{seconds}s")
        }
    }

    /// Add warning message
    #[inline]
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add metadata entry
    #[inline]
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// Execution status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Execution completed successfully
    Success,
    /// Execution failed with error
    Failed,
    /// Execution timed out
    Timeout,
    /// Execution was cancelled
    Cancelled,
    /// Execution is still running
    Running,
}

impl ExecutionStatus {
    /// Get status name as static string for zero allocation
    #[inline]
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Running => "running",
        }
    }
}

/// Execution error types for comprehensive error handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionError {
    /// Execution timed out
    Timeout,
    /// Memory limit exceeded
    MemoryLimitExceeded,
    /// CPU limit exceeded
    CpuLimitExceeded,
    /// Syntax error in code
    SyntaxError { message: String },
    /// Runtime error during execution
    RuntimeError { message: String },
    /// Security violation detected
    SecurityViolation { message: String },
    /// Resource unavailable
    ResourceUnavailable { resource: String },
    /// Network access denied
    NetworkAccessDenied,
    /// Filesystem access denied
    FilesystemAccessDenied,
    /// Compilation failed
    CompilationFailed { message: String },
    /// Invalid language configuration
    InvalidLanguageConfig { message: String },
    /// System error
    SystemError { message: String },
}

impl ExecutionError {
    /// Get error message as string
    #[inline]
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Timeout => "Execution timed out".to_string(),
            Self::MemoryLimitExceeded => "Memory limit exceeded during execution".to_string(),
            Self::CpuLimitExceeded => "CPU limit exceeded during execution".to_string(),
            Self::SyntaxError { message } => format!("Syntax error: {message}"),
            Self::RuntimeError { message } => format!("Runtime error: {message}"),
            Self::SecurityViolation { message } => format!("Security violation: {message}"),
            Self::ResourceUnavailable { resource } => format!("Resource unavailable: {resource}"),
            Self::NetworkAccessDenied => "Network access denied".to_string(),
            Self::FilesystemAccessDenied => "Filesystem access denied".to_string(),
            Self::CompilationFailed { message } => format!("Compilation failed: {message}"),
            Self::InvalidLanguageConfig { message } => {
                format!("Invalid language configuration: {message}")
            }
            Self::SystemError { message } => format!("System error: {message}"),
        }
    }
}
