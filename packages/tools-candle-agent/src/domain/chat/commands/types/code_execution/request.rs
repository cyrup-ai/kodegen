//! Code execution request types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::language::CodeLanguage;
use super::response::OutputFormat;

/// Code execution request with type-safe communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExecutionRequest {
    /// Programming language for execution
    pub language: CodeLanguage,
    /// Source code to execute (owned string allocated once)
    pub code: String,
    /// Reasoning for code execution (owned string allocated once)
    pub reasoning: String,
    /// Execution timeout in seconds
    pub timeout_seconds: u64,
    /// Memory limit in bytes
    pub memory_limit_bytes: u64,
    /// CPU limit as percentage (0-100)
    pub cpu_limit_percent: u8,
    /// Network access permission
    pub network_access: bool,
    /// File system access permission
    pub filesystem_access: bool,
    /// Environment variables to set
    pub environment_variables: HashMap<String, String>,
    /// Working directory for execution
    pub working_directory: Option<String>,
    /// Input data to provide to the executing code
    pub input_data: Option<String>,
    /// Expected output format
    pub expected_output_format: OutputFormat,
}

impl CodeExecutionRequest {
    /// Create a new code execution request with essential fields
    #[inline]
    pub fn new(
        language: CodeLanguage,
        code: impl Into<String>,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            language,
            code: code.into(),
            reasoning: reasoning.into(),
            timeout_seconds: 30,
            memory_limit_bytes: 128 * 1024 * 1024, // 128MB
            cpu_limit_percent: 80,
            network_access: false,
            filesystem_access: false,
            environment_variables: HashMap::new(),
            working_directory: None,
            input_data: None,
            expected_output_format: OutputFormat::Text,
        }
    }

    /// Set timeout - builder pattern for fluent API
    #[inline]
    #[must_use]
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set memory limit - builder pattern for fluent API
    #[inline]
    #[must_use]
    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.memory_limit_bytes = bytes;
        self
    }

    /// Set CPU limit - builder pattern for fluent API
    #[inline]
    #[must_use]
    pub fn with_cpu_limit(mut self, percent: u8) -> Self {
        self.cpu_limit_percent = percent.min(100);
        self
    }

    /// Enable network access - builder pattern for fluent API
    #[inline]
    #[must_use]
    pub fn with_network_access(mut self) -> Self {
        self.network_access = true;
        self
    }

    /// Enable filesystem access - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_filesystem_access(mut self) -> Self {
        self.filesystem_access = true;
        self
    }

    /// Set working directory - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_working_directory(mut self, dir: impl Into<String>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Set input data - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_input_data(mut self, data: impl Into<String>) -> Self {
        self.input_data = Some(data.into());
        self
    }

    /// Set expected output format - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_output_format(mut self, format: OutputFormat) -> Self {
        self.expected_output_format = format;
        self
    }

    /// Add environment variable - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment_variables.insert(key.into(), value.into());
        self
    }
}
