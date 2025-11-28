//! Reasoner tool metadata

use kodegen_mcp_schema::reasoning::{REASONER, ReasonerArgs};
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn reasoner_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: REASONER,
            category: "reasoner",
            description: "Advanced reasoning tool with multiple strategies (Beam Search, MCTS). Processes thoughts step-by-step, supports branching and revision, and tracks best reasoning paths. Use for complex problem-solving that requires exploration of multiple solution approaches.\n\nStrategies:\n- beam_search: Breadth-first exploration (default)\n- mcts: Monte Carlo Tree Search with UCB1\n- mcts_002_alpha: High exploration MCTS variant\n- mcts_002alt_alpha: Length-rewarding MCTS variant\n\nOptional VoyageAI Embedding Integration: Set VOYAGE_API_KEY environment variable to enable semantic coherence scoring.",
            schema: build_schema::<ReasonerArgs>(),
        },
    ]
}
