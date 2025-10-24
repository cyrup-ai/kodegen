//! Advanced reasoning tool with multiple strategies
//!
//! Provides Beam Search, MCTS, and experimental MCTS variants for
//! complex problem-solving with branching and revision support.

mod reasoner;
mod state;
mod strategies;
mod types;

pub use reasoner::Reasoner;
pub use types::*;

use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

// ============================================================================
// TOOL ARGUMENTS
// ============================================================================

/// Arguments for the reasoner tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReasonerArgs {
    /// Current reasoning step
    pub thought: String,

    /// Current step number (1-based)
    #[schemars(range(min = 1))]
    pub thought_number: usize,

    /// Total expected steps
    #[schemars(range(min = 1))]
    pub total_thoughts: usize,

    /// Whether another step is needed
    pub next_thought_needed: bool,

    /// Optional parent thought ID for branching
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// Strategy: beam_search, mcts, mcts_002_alpha, mcts_002alt_alpha
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_type: Option<String>,

    /// Number of top paths to maintain (beam search)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 10))]
    pub beam_width: Option<usize>,

    /// Number of MCTS simulations to run
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 150))]
    pub num_simulations: Option<usize>,
}

/// Prompt arguments (no customization needed)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReasonerPromptArgs {}

// ============================================================================
// TOOL IMPLEMENTATION
// ============================================================================

/// Advanced reasoning tool with multiple strategies
#[derive(Clone)]
pub struct SequentialThinkingReasonerTool {
    reasoner: Arc<Reasoner>,
}

impl SequentialThinkingReasonerTool {
    /// Create new reasoner tool with optional cache size
    pub fn new(cache_size: Option<usize>) -> Self {
        Self {
            reasoner: Arc::new(Reasoner::new(cache_size)),
        }
    }
}

impl Tool for SequentialThinkingReasonerTool {
    type Args = ReasonerArgs;
    type PromptArgs = ReasonerPromptArgs;

    fn name() -> &'static str {
        "sequential_thinking_reasoner"
    }

    fn description() -> &'static str {
        "Advanced reasoning tool with multiple strategies (Beam Search, MCTS). \
         Processes thoughts step-by-step, supports branching and revision, \
         and tracks best reasoning paths. Use for complex problem-solving \
         that requires exploration of multiple solution approaches.\n\n\
         Strategies:\n\
         - beam_search: Breadth-first exploration (default)\n\
         - mcts: Monte Carlo Tree Search with UCB1\n\
         - mcts_002_alpha: High exploration MCTS variant\n\
         - mcts_002alt_alpha: Length-rewarding MCTS variant\n\n\
         Optional VoyageAI Embedding Integration:\n\
         Set VOYAGE_API_KEY environment variable to enable semantic coherence scoring."
    }

    fn read_only() -> bool {
        true // Only tracks reasoning state, no external modifications
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Convert args to internal ReasoningRequest
        let request = ReasoningRequest {
            thought: args.thought,
            thought_number: args.thought_number,
            total_thoughts: args.total_thoughts,
            next_thought_needed: args.next_thought_needed,
            parent_id: args.parent_id,
            strategy_type: args.strategy_type,
            beam_width: args.beam_width,
            num_simulations: args.num_simulations,
        };

        // Process thought via reasoner (already async)
        let response = self.reasoner.process_thought(request).await;

        // Convert to JSON
        serde_json::to_value(&response)
            .map_err(|e| McpError::Other(anyhow::anyhow!("Serialization failed: {}", e)))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text(
                    "How do I use the sequential_thinking_reasoner tool with different strategies?",
                ),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "The sequential_thinking_reasoner tool supports 4 reasoning strategies:\n\n\
                     1. **beam_search** (default): Explores top N paths simultaneously\n\
                        - Use for breadth-first exploration\n\
                        - Set beamWidth to control path count (default: 3)\n\n\
                     2. **mcts**: Monte Carlo Tree Search with UCB1\n\
                        - Use for exploration-exploitation balance\n\
                        - Set numSimulations to control search depth (default: 50)\n\n\
                     3. **mcts_002_alpha**: MCTS with higher exploration\n\
                        - Use for creative problem-solving\n\
                        - 10% exploration boost\n\n\
                     4. **mcts_002alt_alpha**: MCTS with length bonus\n\
                        - Use for detailed reasoning\n\
                        - Rewards thorough explanations\n\n\
                     Example usage:\n\
                     ```json\n\
                     {\n\
                       \"thought\": \"Analyzing algorithm complexity\",\n\
                       \"thoughtNumber\": 1,\n\
                       \"totalThoughts\": 5,\n\
                       \"nextThoughtNeeded\": true,\n\
                       \"strategyType\": \"mcts\",\n\
                       \"numSimulations\": 100\n\
                     }\n\
                     ```\n\n\
                     The tool returns:\n\
                     - nodeId: Unique identifier for this thought\n\
                     - score: Quality score (0.0-1.0)\n\
                     - depth: Current reasoning depth\n\
                     - strategyUsed: Which strategy was applied\n\
                     - bestScore: Highest score in current path\n\n\
                     Optional: Set VOYAGE_API_KEY environment variable to enable \n\
                     semantic coherence scoring using VoyageAI embeddings.",
                ),
            },
        ])
    }
}
