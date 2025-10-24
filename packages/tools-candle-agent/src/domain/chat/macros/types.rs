//! Core type definitions for macro actions and states

use crate::domain::chat::commands::ImmutableChatCommand;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Macro action representing a single recorded operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MacroAction {
    /// Send a message with content
    SendMessage {
        /// Message content to send
        content: String,
        /// Type of message being sent
        message_type: String,
        /// Timestamp when action was recorded
        timestamp: Duration,
    },
    /// Execute a command
    ExecuteCommand {
        /// Command to execute
        command: ImmutableChatCommand,
        /// Timestamp when action was recorded
        timestamp: Duration,
    },
    /// Wait for a specified duration
    Wait {
        /// Duration to wait
        duration: Duration,
        /// Timestamp when action was recorded
        timestamp: Duration,
    },
    /// Set a variable value
    SetVariable {
        /// Variable name to set
        name: String,
        /// Value to assign to the variable
        value: String,
        /// Timestamp when action was recorded
        timestamp: Duration,
    },
    /// Conditional execution based on variable
    Conditional {
        /// Condition expression to evaluate
        condition: String,
        /// Actions to execute if condition is true
        then_actions: Vec<MacroAction>,
        /// Actions to execute if condition is false
        else_actions: Option<Vec<MacroAction>>,
        /// Timestamp when action was recorded
        timestamp: Duration,
    },
    /// Loop execution
    Loop {
        /// Number of iterations to perform
        iterations: u32,
        /// Actions to execute in each iteration
        actions: Vec<MacroAction>,
        /// Timestamp when action was recorded
        timestamp: Duration,
    },
}

/// Macro recording state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroRecordingState {
    /// Not recording
    Idle,
    /// Currently recording
    Recording,
    /// Recording paused
    Paused,
    /// Recording completed
    Completed,
}

/// Macro playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroPlaybackState {
    /// Not playing
    Idle,
    /// Currently playing
    Playing,
    /// Playback paused
    Paused,
    /// Playback completed
    Completed,
    /// Playback failed
    Failed,
}

/// Token types for condition evaluation
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CondToken {
    /// Literal value (identifier, number, or string)
    Value(String),

    // Comparison operators (precedence 80)
    /// Equality operator ==
    Eq,
    /// Inequality operator !=
    Neq,
    /// Less than operator <
    Lt,
    /// Greater than operator >
    Gt,
    /// Less than or equal <=
    Leq,
    /// Greater than or equal >=
    Geq,

    // Logical operators
    /// Logical AND &&
    And, // precedence 75
    /// Logical OR ||
    Or, // precedence 70
    /// Logical NOT !
    Not, // precedence 110

    // Grouping
    /// Left parenthesis (
    LParen,
    /// Right parenthesis )
    RParen,
}
