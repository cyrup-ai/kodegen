//! Command execution event types for streaming with zero allocation patterns
//!
//! Provides event variants and constructors for blazing-fast event streaming.

use serde::{Deserialize, Serialize};

use super::super::commands::{CommandExecutionResult, ImmutableChatCommand, OutputType};
use super::super::metadata::ResourceUsage;
use crate::domain::util::unix_timestamp_micros;

/// Command execution event for streaming with zero allocation where possible
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandEvent {
    /// Command started executing
    Started {
        /// The command that was started
        command: ImmutableChatCommand,
        /// Unique identifier for this execution
        execution_id: u64,
        /// Start time in microseconds since epoch
        timestamp_us: u64,
    },
    /// Command execution progress
    Progress {
        /// Unique identifier for this execution
        execution_id: u64,
        /// Progress percentage (0-100)
        progress: f32,
        /// Status message
        message: String,
        /// Timestamp
        timestamp: u64,
    },
    /// Command produced output
    Output {
        /// Unique identifier for this execution
        execution_id: u64,
        /// Output content
        content: String,
        /// Type/format of the output
        output_type: OutputType,
        /// Current timestamp in microseconds since epoch
        timestamp_us: u64,
    },
    /// Command completed successfully
    Completed {
        /// Unique identifier for this execution
        execution_id: u64,
        /// Final result of the execution
        result: CommandExecutionResult,
        /// Total execution time in microseconds
        duration_us: u64,
        /// Final resource usage
        resource_usage: ResourceUsage,
        /// Completion timestamp in microseconds since epoch
        timestamp_us: u64,
    },
    /// Command execution failed
    Failed {
        /// Unique identifier for this execution
        execution_id: u64,
        /// Error message (owned string allocated once)
        error: String,
        /// Error code for categorization
        error_code: u32,
        /// Total execution time in microseconds
        duration_us: u64,
        /// Resource usage at failure
        resource_usage: ResourceUsage,
        /// Failure timestamp in microseconds since epoch
        timestamp_us: u64,
    },
    /// Command execution was cancelled
    Cancelled {
        /// Unique identifier for this execution
        execution_id: u64,
        /// Cancellation reason (owned string allocated once)
        reason: String,
        /// Partial execution time in microseconds
        duration_us: u64,
        /// Resource usage at cancellation
        resource_usage: ResourceUsage,
        /// Cancellation timestamp in microseconds since epoch
        timestamp_us: u64,
    },
    /// Command execution warning
    Warning {
        /// Unique identifier for this execution
        execution_id: u64,
        /// Warning message (owned string allocated once)
        message: String,
        /// Warning severity (1=low, 2=medium, 3=high, 4=critical)
        severity: u8,
        /// Warning timestamp in microseconds since epoch
        timestamp_us: u64,
    },
    /// Resource threshold exceeded
    ResourceAlert {
        /// Unique identifier for this execution
        execution_id: u64,
        /// Resource type that exceeded threshold (owned string allocated once)
        resource_type: String,
        /// Current usage value
        current_value: u64,
        /// Threshold value that was exceeded
        threshold_value: u64,
        /// Alert timestamp in microseconds since epoch
        timestamp_us: u64,
    },
}

impl CommandEvent {
    /// Create started event with zero allocation constructor
    #[inline]
    #[must_use]
    pub fn started(command: ImmutableChatCommand, execution_id: u64) -> Self {
        Self::Started {
            command,
            execution_id,
            timestamp_us: Self::current_timestamp_us(),
        }
    }

    /// Create progress event with zero allocation constructor
    #[inline]
    #[must_use]
    pub fn progress(execution_id: u64, progress: f32, message: String) -> Self {
        Self::Progress {
            execution_id,
            progress,
            message,
            timestamp: Self::current_timestamp_us(),
        }
    }

    /// Create output event with zero allocation constructor
    #[inline]
    pub fn output(execution_id: u64, output: impl Into<String>, output_type: OutputType) -> Self {
        Self::Output {
            execution_id,
            content: output.into(),
            output_type,
            timestamp_us: Self::current_timestamp_us(),
        }
    }

    /// Create completed event with zero allocation constructor
    #[inline]
    #[must_use]
    pub fn completed(
        execution_id: u64,
        result: CommandExecutionResult,
        duration_us: u64,
        resource_usage: ResourceUsage,
    ) -> Self {
        Self::Completed {
            execution_id,
            result,
            duration_us,
            resource_usage,
            timestamp_us: Self::current_timestamp_us(),
        }
    }

    /// Create failed event with zero allocation constructor
    #[inline]
    pub fn failed(
        execution_id: u64,
        error: impl Into<String>,
        error_code: u32,
        duration_us: u64,
        resource_usage: ResourceUsage,
    ) -> Self {
        Self::Failed {
            execution_id,
            error: error.into(),
            error_code,
            duration_us,
            resource_usage,
            timestamp_us: Self::current_timestamp_us(),
        }
    }

    /// Create cancelled event with zero allocation constructor
    #[inline]
    pub fn cancelled(
        execution_id: u64,
        reason: impl Into<String>,
        duration_us: u64,
        resource_usage: ResourceUsage,
    ) -> Self {
        Self::Cancelled {
            execution_id,
            reason: reason.into(),
            duration_us,
            resource_usage,
            timestamp_us: Self::current_timestamp_us(),
        }
    }

    /// Create warning event with zero allocation constructor
    #[inline]
    pub fn warning(execution_id: u64, message: impl Into<String>, severity: u8) -> Self {
        Self::Warning {
            execution_id,
            message: message.into(),
            severity,
            timestamp_us: Self::current_timestamp_us(),
        }
    }

    /// Create resource alert event with zero allocation constructor
    #[inline]
    pub fn resource_alert(
        execution_id: u64,
        resource_type: impl Into<String>,
        current_value: u64,
        threshold_value: u64,
    ) -> Self {
        Self::ResourceAlert {
            execution_id,
            resource_type: resource_type.into(),
            current_value,
            threshold_value,
            timestamp_us: Self::current_timestamp_us(),
        }
    }

    /// Get current timestamp in microseconds since epoch
    #[inline]
    fn current_timestamp_us() -> u64 {
        unix_timestamp_micros()
    }

    /// Get execution ID from any event type
    #[inline]
    #[must_use]
    pub const fn execution_id(&self) -> u64 {
        match self {
            Self::Started { execution_id, .. }
            | Self::Progress { execution_id, .. }
            | Self::Output { execution_id, .. }
            | Self::Completed { execution_id, .. }
            | Self::Failed { execution_id, .. }
            | Self::Cancelled { execution_id, .. }
            | Self::Warning { execution_id, .. }
            | Self::ResourceAlert { execution_id, .. } => *execution_id,
        }
    }

    /// Get timestamp from any event type
    #[inline]
    #[must_use]
    pub const fn timestamp_us(&self) -> u64 {
        match self {
            Self::Started { timestamp_us, .. }
            | Self::Output { timestamp_us, .. }
            | Self::Completed { timestamp_us, .. }
            | Self::Failed { timestamp_us, .. }
            | Self::Cancelled { timestamp_us, .. }
            | Self::Warning { timestamp_us, .. }
            | Self::ResourceAlert { timestamp_us, .. } => *timestamp_us,
            Self::Progress { timestamp, .. } => *timestamp,
        }
    }

    /// Check if event indicates completion (success, failure, or cancellation)
    #[inline]
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::Cancelled { .. }
        )
    }

    /// Check if event indicates success
    #[inline]
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    /// Check if event indicates failure
    #[inline]
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Check if event indicates cancellation
    #[inline]
    #[must_use]
    pub const fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled { .. })
    }

    /// Get event severity level for filtering and monitoring
    #[inline]
    #[must_use]
    pub const fn severity(&self) -> u8 {
        match self {
            Self::Started { .. } | Self::Progress { .. } | Self::Output { .. } => 0,
            Self::Completed { .. } => 1,
            Self::Warning { severity, .. } => *severity,
            Self::ResourceAlert { .. } | Self::Cancelled { .. } => 2,
            Self::Failed { .. } => 3,
        }
    }
}
