//! Sequential thinking tool metadata

use kodegen_mcp_schema::reasoning::{SEQUENTIAL_THINKING, SequentialThinkingArgs};
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn sequential_thinking_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: SEQUENTIAL_THINKING,
            category: SEQUENTIAL_THINKING,
            description: "A detailed tool for dynamic and reflective problem-solving through thoughts. This tool helps analyze problems through a flexible thinking process that can adapt and evolve. Each thought can build on, question, or revise previous insights as understanding deepens.",
            schema: build_schema::<SequentialThinkingArgs>(),
        },
    ]
}
