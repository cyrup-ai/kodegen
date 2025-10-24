mod core;
pub mod prompts;
mod views;

use serde::{Deserialize, Serialize};

pub use core::{Agent, AgentConfig, PromptConfig};
pub use prompts::{AgentMessagePrompt, SystemPrompt};
pub use views::{ActionView, BrowserStateView, HistoryView, StepView};

use thiserror::Error;

/// Action model for agent protocol - represents an action to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionModel {
    pub action: String,
    pub parameters: std::collections::HashMap<String, String>,
}

/// Result of executing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: String,
    pub success: bool,
    pub extracted_content: Option<String>,
    pub error: Option<String>,
}

/// Response from browser_extract_text MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserExtractTextResponse {
    pub success: bool,
    pub text: String,
    pub length: usize,
    #[serde(default)]
    pub selector: Option<String>,
    pub source: String,
    pub message: String,
}

/// Response from browser_screenshot MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserScreenshotResponse {
    pub success: bool,
    pub image: String,
    pub format: String,
    pub size_bytes: usize,
    #[serde(default)]
    pub selector: Option<String>,
    pub message: String,
}

/// Agent LLM protocol-compliant response schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLLMResponse {
    pub current_state: CurrentState,
    pub action: Vec<ActionModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentState {
    pub prev_action_evaluation: String,
    pub important_contents: String,
    pub task_progress: String,
    pub future_plans: String,
    pub thought: String,
    pub summary: String,
}

/// Error type for agent operations
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Browser error: {0}")]
    BrowserError(String),

    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    #[error("Step failed: {0}")]
    StepFailed(String),

    #[error("Agent stopped")]
    Stopped,

    #[error("Channel closed: {0}")]
    ChannelClosed(String),

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub current_state: CurrentState,
    pub action: Vec<ActionModel>,
}

/// An entry in the agent history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHistory {
    pub step: usize,
    pub output: AgentOutput,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub is_complete: bool,
}

/// A list of agent history entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHistoryList {
    pub steps: Vec<AgentHistory>,
}

impl AgentHistoryList {
    /// Create a new agent history list
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Add a step to the history (not marking completion)
    pub fn add_step(&mut self, output: AgentOutput) {
        let step = AgentHistory {
            step: self.steps.len(),
            output,
            timestamp: chrono::Utc::now(),
            is_complete: false,
        };
        self.steps.push(step);
    }

    /// Add a step to the history, with explicit completion flag
    pub fn add_step_with_completion(&mut self, output: AgentOutput, is_complete: bool) {
        let step = AgentHistory {
            step: self.steps.len(),
            output,
            timestamp: chrono::Utc::now(),
            is_complete,
        };
        self.steps.push(step);
    }

    /// Returns true if any step marks the task as complete
    pub fn is_complete(&self) -> bool {
        self.steps.iter().any(|s| s.is_complete)
    }

    /// Returns the final result if the task is complete
    pub fn final_result(&self) -> Option<String> {
        self.steps.iter().rev().find(|s| s.is_complete).map(|last| {
            format!(
                "Task completed at step {}. Summary: {}",
                last.step, last.output.current_state.summary
            )
        })
    }
}

impl Default for AgentHistoryList {
    fn default() -> Self {
        Self::new()
    }
}
