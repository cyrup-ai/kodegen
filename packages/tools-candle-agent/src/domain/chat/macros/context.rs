//! Execution context and metadata structures

use super::types::MacroAction;
use crate::domain::chat::conversation::CandleStreamingConversation;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Macro execution context with variable substitution
#[derive(Debug, Clone)]
pub struct MacroExecutionContext {
    /// Variables available for substitution during execution
    pub variables: HashMap<String, String>,
    /// Unique identifier for this execution
    pub execution_id: Uuid,
    /// When execution started
    pub start_time: DateTime<Utc>,
    /// Current action index being executed
    pub current_action: usize,
    /// Stack of nested loop contexts
    pub loop_stack: Vec<LoopContext>,
    /// Optional conversation for message sending
    pub conversation: Option<Arc<RwLock<CandleStreamingConversation>>>,
}

impl Default for MacroExecutionContext {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            execution_id: Uuid::new_v4(),
            start_time: Utc::now(),
            current_action: 0,
            loop_stack: Vec::new(),
            conversation: None,
        }
    }
}

/// Map message type string to appropriate conversation method
pub(crate) async fn send_message_to_conversation(
    conversation: &Arc<RwLock<CandleStreamingConversation>>,
    content: String,
    message_type: &str,
) -> Result<(), String> {
    let mut conv = conversation.write().await;

    let result = match message_type.to_lowercase().as_str() {
        "assistant" => conv.add_assistant_message(content),
        "system" => conv.add_system_message(content),
        _ => conv.add_user_message(content), // Default to user (includes "user" case)
    };

    result
        .map(|_| ())
        .map_err(|e| format!("Failed to add message to conversation: {e}"))
}

/// Loop execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopContext {
    /// Current iteration number (0-based)
    pub iteration: u32,
    /// Maximum number of iterations for this loop
    pub max_iterations: u32,
    /// Index of the first action in the loop
    pub start_action: usize,
    /// Index of the last action in the loop
    pub end_action: usize,
}

/// Macro metadata and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroMetadata {
    /// Unique identifier for the macro
    pub id: Uuid,
    /// Human-readable name of the macro
    pub name: String,
    /// Description of what the macro does
    pub description: String,
    /// When the macro was first created
    pub created_at: Duration,
    /// When the macro was last modified
    pub updated_at: Duration,
    /// Version number of the macro
    pub version: u32,
    /// Tags for categorizing and searching macros
    pub tags: Vec<String>,
    /// Author who created the macro
    pub author: String,
    /// Number of times this macro has been executed
    pub execution_count: u64,
    /// When the macro was last executed
    pub last_execution: Option<Duration>,
    /// Average execution duration
    pub average_duration: Duration,
    /// Success rate of executions (0.0 to 1.0)
    pub success_rate: f64,
    /// Category this macro belongs to
    pub category: String,
    /// Whether this macro is private to the user
    pub is_private: bool,
}

/// Complete macro definition with actions and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMacro {
    /// Metadata and statistics for the macro
    pub metadata: MacroMetadata,
    /// Sequence of actions to execute
    pub actions: Vec<MacroAction>,
    /// Default variables for macro execution
    pub variables: HashMap<String, String>,
    /// Triggers that can activate this macro
    pub triggers: Vec<String>,
    /// Conditions that must be met for execution
    pub conditions: Vec<String>,
    /// Other macros this macro depends on
    pub dependencies: Vec<String>,
    /// Configuration for how this macro executes
    pub execution_config: MacroExecutionConfig,
}

/// Macro execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroExecutionConfig {
    /// Maximum time allowed for macro execution
    pub max_execution_time: Duration,
    /// Number of times to retry on failure
    pub retry_count: u32,
    /// Delay between retry attempts
    pub retry_delay: Duration,
    /// Whether to abort execution on any error
    pub abort_on_error: bool,
    /// Whether actions can be executed in parallel
    pub parallel_execution: bool,
    /// Execution priority (0-255, higher = more priority)
    pub priority: u8,
    /// Resource limits for macro execution
    pub resource_limits: ResourceLimits,
}

impl Default for MacroExecutionConfig {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(300), // 5 minutes
            retry_count: 3,
            retry_delay: Duration::from_millis(1000),
            abort_on_error: false,
            parallel_execution: false,
            priority: 5,
            resource_limits: ResourceLimits::default(),
        }
    }
}

/// Resource limits for macro execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in megabytes
    pub max_memory_mb: u32,
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_percent: u8,
    /// Maximum number of network requests allowed
    pub max_network_requests: u32,
    /// Maximum number of file operations allowed
    pub max_file_operations: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 100,
            max_cpu_percent: 25,
            max_network_requests: 50,
            max_file_operations: 20,
        }
    }
}
