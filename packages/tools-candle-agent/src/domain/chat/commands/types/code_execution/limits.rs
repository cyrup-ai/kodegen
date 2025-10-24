//! Resource limits for code execution

use serde::{Deserialize, Serialize};

use super::errors::ValidationError;
use super::request::CodeExecutionRequest;

/// Resource limits for code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum execution time in seconds
    pub max_execution_time_seconds: u64,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Maximum CPU usage as percentage (0-100)
    pub max_cpu_percent: u8,
    /// Maximum number of file operations
    pub max_file_operations: u32,
    /// Maximum number of network requests
    pub max_network_requests: u32,
    /// Maximum output size in bytes
    pub max_output_size_bytes: u64,
    /// Maximum number of processes/threads
    pub max_processes: u32,
}

impl Default for ResourceLimits {
    /// Create default resource limits
    #[inline]
    fn default() -> Self {
        Self {
            max_execution_time_seconds: 30,
            max_memory_bytes: 128 * 1024 * 1024, // 128MB
            max_cpu_percent: 80,
            max_file_operations: 100,
            max_network_requests: 0,            // No network by default
            max_output_size_bytes: 1024 * 1024, // 1MB
            max_processes: 1,
        }
    }
}

impl ResourceLimits {
    /// Create resource limits for analysis workloads
    #[inline]
    #[must_use]
    pub fn analysis_workload() -> Self {
        Self {
            max_execution_time_seconds: 60,
            max_memory_bytes: 512 * 1024 * 1024, // 512MB
            max_cpu_percent: 90,
            max_file_operations: 1000,
            max_network_requests: 0,
            max_output_size_bytes: 10 * 1024 * 1024, // 10MB
            max_processes: 1,
        }
    }

    /// Create resource limits for processing workloads
    #[inline]
    #[must_use]
    pub fn processing_workload() -> Self {
        Self {
            max_execution_time_seconds: 45,
            max_memory_bytes: 256 * 1024 * 1024, // 256MB
            max_cpu_percent: 85,
            max_file_operations: 500,
            max_network_requests: 10,
            max_output_size_bytes: 5 * 1024 * 1024, // 5MB
            max_processes: 2,
        }
    }

    /// Create resource limits for computation workloads
    #[inline]
    #[must_use]
    pub fn computation_workload() -> Self {
        Self {
            max_execution_time_seconds: 90,
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_cpu_percent: 95,
            max_file_operations: 100,
            max_network_requests: 0,
            max_output_size_bytes: 2 * 1024 * 1024, // 2MB
            max_processes: 1,
        }
    }

    /// Create resource limits for automation workloads
    #[inline]
    #[must_use]
    pub fn automation_workload() -> Self {
        Self {
            max_execution_time_seconds: 120,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            max_cpu_percent: 70,
            max_file_operations: 2000,
            max_network_requests: 0,
            max_output_size_bytes: 512 * 1024, // 512KB
            max_processes: 5,
        }
    }

    /// Validate execution request against these limits
    ///
    /// # Errors
    ///
    /// Returns `ValidationError` if:
    /// - Request timeout exceeds `max_execution_time_seconds`
    /// - Request memory limit exceeds `max_memory_bytes`
    #[inline]
    pub fn validate_request(&self, request: &CodeExecutionRequest) -> Result<(), ValidationError> {
        if request.timeout_seconds > self.max_execution_time_seconds {
            return Err(ValidationError::InvalidTimeout {
                timeout: request.timeout_seconds,
                max_timeout: self.max_execution_time_seconds,
            });
        }

        if request.memory_limit_bytes > self.max_memory_bytes {
            return Err(ValidationError::InvalidMemoryLimit {
                memory: request.memory_limit_bytes,
                max_memory: self.max_memory_bytes,
            });
        }

        if request.cpu_limit_percent > self.max_cpu_percent {
            return Err(ValidationError::InvalidCpuLimit {
                cpu: request.cpu_limit_percent,
                max_cpu: self.max_cpu_percent,
            });
        }

        Ok(())
    }
}
