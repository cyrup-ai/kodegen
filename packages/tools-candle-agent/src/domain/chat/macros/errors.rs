//! Error types and result enums

use cyrup_sugars::prelude::MessageChunk;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Result of macro action execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ActionExecutionResult {
    /// Action executed successfully
    #[default]
    Success,
    /// Wait for specified duration before continuing
    Wait(Duration),
    /// Skip to the specified action index
    SkipToAction(usize),
    /// Execution failed with error message
    Error(String),
}

impl MessageChunk for ActionExecutionResult {
    fn bad_chunk(error: String) -> Self {
        ActionExecutionResult::Error(error)
    }

    fn error(&self) -> Option<&str> {
        match self {
            ActionExecutionResult::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Result of macro playback operation
#[derive(Debug)]
pub enum MacroPlaybackResult {
    /// Single action was executed successfully
    ActionExecuted,
    /// Macro playback completed successfully
    Completed,
    /// Macro playback failed
    Failed,
    /// Playback session is not active
    SessionNotActive,
}

/// Macro system errors
#[derive(Debug, thiserror::Error)]
pub enum MacroSystemError {
    /// Requested macro session was not found
    #[error("Recording session not found")]
    SessionNotFound,
    /// Macro recording is not currently active
    #[error("Recording not active")]
    RecordingNotActive,
    /// Requested macro does not exist
    #[error("Macro not found")]
    MacroNotFound,
    /// System time operation failed
    #[error("System time error")]
    SystemTimeError,
    /// Error occurred during macro execution
    #[error("Execution error: {0}")]
    ExecutionError(String),
    /// Macro validation failed
    #[error("Validation error: {0}")]
    ValidationError(String),
}
