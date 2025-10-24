//! Streaming command executor with atomic state tracking for zero allocation
//!
//! Provides command execution lifecycle management with atomic operations.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::super::commands::{CommandExecutionResult, ImmutableChatCommand, OutputType};
use super::super::metadata::ResourceUsage;
use super::event_types::CommandEvent;
use super::stats::CommandExecutorStats;

/// Streaming command executor with atomic state tracking for zero allocation
pub struct StreamingCommandExecutor {
    /// Execution counter (atomic) - thread-safe incrementing
    pub(super) execution_counter: AtomicU64,
    /// Active executions (atomic) - current running count
    pub(super) active_executions: AtomicUsize,
    /// Total executions (atomic) - lifetime total
    pub(super) total_executions: AtomicU64,
    /// Successful executions (atomic) - success count
    pub(super) successful_executions: AtomicU64,
    /// Failed executions (atomic) - failure count
    pub(super) failed_executions: AtomicU64,
    /// Event stream sender for broadcasting events
    pub(super) event_sender: Option<tokio::sync::mpsc::UnboundedSender<CommandEvent>>,
}

impl StreamingCommandExecutor {
    /// Create new streaming command executor with zero allocation
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            execution_counter: AtomicU64::new(0),
            active_executions: AtomicUsize::new(0),
            total_executions: AtomicU64::new(0),
            successful_executions: AtomicU64::new(0),
            failed_executions: AtomicU64::new(0),
            event_sender: None,
        }
    }

    /// Create with event sender for streaming events
    #[inline]
    #[must_use]
    pub fn with_event_sender(
        event_sender: tokio::sync::mpsc::UnboundedSender<CommandEvent>,
    ) -> Self {
        Self {
            execution_counter: AtomicU64::new(0),
            active_executions: AtomicUsize::new(0),
            total_executions: AtomicU64::new(0),
            successful_executions: AtomicU64::new(0),
            failed_executions: AtomicU64::new(0),
            event_sender: Some(event_sender),
        }
    }

    /// Start new command execution and return unique ID
    #[inline]
    pub fn start_execution(&self, command: &ImmutableChatCommand) -> u64 {
        let execution_id = self.execution_counter.fetch_add(1, Ordering::Relaxed);
        self.active_executions.fetch_add(1, Ordering::Relaxed);
        self.total_executions.fetch_add(1, Ordering::Relaxed);

        // Send started event if sender available
        if let Some(ref sender) = self.event_sender {
            let event = CommandEvent::started(command.clone(), execution_id);
            if sender.send(event).is_err() {
                // Event channel closed, continue execution
            }
        }

        execution_id
    }

    /// Mark execution as completed successfully
    #[inline]
    pub fn complete_execution(
        &self,
        execution_id: u64,
        result: CommandExecutionResult,
        duration_us: u64,
        resource_usage: ResourceUsage,
    ) {
        self.active_executions.fetch_sub(1, Ordering::Relaxed);
        self.successful_executions.fetch_add(1, Ordering::Relaxed);

        // Send completed event if sender available
        if let Some(ref sender) = self.event_sender {
            let event = CommandEvent::completed(execution_id, result, duration_us, resource_usage);
            if sender.send(event).is_err() {
                // Event channel closed, continue
            }
        }
    }

    /// Mark execution as failed
    #[inline]
    pub fn fail_execution(
        &self,
        execution_id: u64,
        error: impl Into<String>,
        error_code: u32,
        duration_us: u64,
        resource_usage: ResourceUsage,
    ) {
        self.active_executions.fetch_sub(1, Ordering::Relaxed);
        self.failed_executions.fetch_add(1, Ordering::Relaxed);

        // Send failed event if sender available
        if let Some(ref sender) = self.event_sender {
            let event =
                CommandEvent::failed(execution_id, error, error_code, duration_us, resource_usage);
            if sender.send(event).is_err() {
                // Event channel closed, continue
            }
        }
    }

    /// Cancel command execution
    #[inline]
    pub fn cancel_execution(
        &self,
        execution_id: u64,
        reason: impl Into<String>,
        duration_us: u64,
        resource_usage: ResourceUsage,
    ) {
        self.active_executions.fetch_sub(1, Ordering::Relaxed);

        // Send cancelled event if sender available
        if let Some(ref sender) = self.event_sender {
            let event = CommandEvent::cancelled(execution_id, reason, duration_us, resource_usage);
            if sender.send(event).is_err() {
                // Event channel closed, continue
            }
        }
    }

    /// Send progress update
    #[inline]
    pub fn send_progress(&self, execution_id: u64, progress: f32, message: Option<String>) {
        if let Some(ref sender) = self.event_sender {
            let event = CommandEvent::progress(execution_id, progress, message.unwrap_or_default());
            if sender.send(event).is_err() {
                // Event channel closed, continue
            }
        }
    }

    /// Send output data
    #[inline]
    pub fn send_output(
        &self,
        execution_id: u64,
        output: impl Into<String>,
        output_type: OutputType,
    ) {
        if let Some(ref sender) = self.event_sender {
            let event = CommandEvent::output(execution_id, output, output_type);
            if sender.send(event).is_err() {
                // Event channel closed, continue
            }
        }
    }

    /// Send warning
    #[inline]
    pub fn send_warning(&self, execution_id: u64, message: impl Into<String>, severity: u8) {
        if let Some(ref sender) = self.event_sender {
            let event = CommandEvent::warning(execution_id, message, severity);
            if sender.send(event).is_err() {
                // Event channel closed, continue
            }
        }
    }

    /// Get execution statistics (atomic reads) - zero allocation
    #[inline]
    pub fn stats(&self) -> CommandExecutorStats {
        CommandExecutorStats {
            active_executions: self.active_executions.load(Ordering::Relaxed) as u64,
            total_executions: self.total_executions.load(Ordering::Relaxed),
            successful_executions: self.successful_executions.load(Ordering::Relaxed),
            failed_executions: self.failed_executions.load(Ordering::Relaxed),
        }
    }

    /// Get next execution ID without starting execution
    #[inline]
    pub fn peek_next_execution_id(&self) -> u64 {
        self.execution_counter.load(Ordering::Relaxed)
    }
}

impl Default for StreamingCommandExecutor {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
