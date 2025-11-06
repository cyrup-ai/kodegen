//! AI and reasoning tools: claude_agent, candle_agent, reasoner, sequential_thinking, prompt

use kodegen_mcp_schema::*;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn ai_reasoning_tools() -> Vec<ToolMetadata> {
    vec![
        // CLAUDE_AGENT (5 tools)
        ToolMetadata {
            name: "list_claude_agents",
            category: "claude_agent",
            description: "List all active and completed agent sessions with status and output preview. Shows working indicator (true if actively processing), turn count, run...",
            schema: build_schema::<claude_agent::ListClaudeAgentsArgs>(),
        },
        ToolMetadata {
            name: "read_claude_agent_output",
            category: "claude_agent",
            description: "Read paginated output from an agent session. Returns messages with working indicator. Use offset/length for pagination (offset=0 for start, negativ...",
            schema: build_schema::<claude_agent::ReadClaudeAgentOutputArgs>(),
        },
        ToolMetadata {
            name: "send_claude_agent_prompt",
            category: "claude_agent",
            description: "Send a follow-up prompt to an active agent session. Continues the conversation with new instructions or questions. Use read_claude_agent_output to ...",
            schema: build_schema::<claude_agent::SendClaudeAgentPromptArgs>(),
        },
        ToolMetadata {
            name: "spawn_claude_agent",
            category: "claude_agent",
            description: "Spawn one or more Claude agent sessions for parallel task delegation. Each agent gets identical configuration and can work independently. Use worke...",
            schema: build_schema::<claude_agent::SpawnClaudeAgentArgs>(),
        },
        ToolMetadata {
            name: "terminate_claude_agent_session",
            category: "claude_agent",
            description: "Gracefully terminate an agent session. Closes the ClaudeSDKClient connection, returns final statistics (turn count, message count, runtime), and mo...",
            schema: build_schema::<claude_agent::TerminateClaudeAgentSessionArgs>(),
        },
        // CANDLE_AGENT (4 tools)
        ToolMetadata {
            name: "memorize",
            category: "candle_agent",
            description: "Store content in a named memory library with automatic embedding generation. The memory will be tagged with the library name and can be retrieved later using recall(). Each library is a separate namespace for organizing memories.",
            schema: build_schema::<claude_agent::MemorizeArgs>(),
        },
        ToolMetadata {
            name: "check_memorize_status",
            category: "candle_agent",
            description: "Check the status of an async memorize operation started with memorize().\n\nReturns current status, progress information, and memory_id when complete.\n\nStatus values:\n- IN_PROGRESS: Task is still running (loading content, generating embeddings, storing)\n- COMPLETED: Task finished successfully (memory_id available)\n- FAILED: Task failed (error message available)\n\nPoll this repeatedly (with delays) until status is COMPLETED or FAILED.\nProgress includes current stage (Loading content, Generating embeddings, Storing in database)\nand file counts for multi-file operations.",
            schema: build_schema::<claude_agent::CheckMemorizeStatusArgs>(),
        },
        ToolMetadata {
            name: "recall",
            category: "candle_agent",
            description: "Retrieve relevant memories from a library using semantic search. Searches for content similar to the provided context and returns the most relevant results. Uses vector similarity (cosine) to find semantically related memories.",
            schema: build_schema::<claude_agent::RecallArgs>(),
        },
        ToolMetadata {
            name: "list_memory_libraries",
            category: "candle_agent",
            description: "List all unique memory library names that have been created. Returns a list of all libraries that contain at least one memory. Use this to discover what libraries are available for recall.",
            schema: build_schema::<claude_agent::ListMemoryLibrariesArgs>(),
        },
        // PROMPT (4 tools)
        ToolMetadata {
            name: "add_prompt",
            category: "prompt",
            description: "Create a new prompt template. The content must include YAML frontmatter with metadata (title, description, categories, author) followed by the temp...",
            schema: build_schema::<prompt::AddPromptArgs>(),
        },
        ToolMetadata {
            name: "delete_prompt",
            category: "prompt",
            description: "Delete a prompt template. Requires confirm=true for safety. This action cannot be undone. Default prompts can be deleted but will be recreated on n...",
            schema: build_schema::<prompt::DeletePromptArgs>(),
        },
        ToolMetadata {
            name: "edit_prompt",
            category: "prompt",
            description: "Edit an existing prompt template. Provide the prompt name and complete new content (including YAML frontmatter). The content is validated before sa...",
            schema: build_schema::<prompt::EditPromptArgs>(),
        },
        ToolMetadata {
            name: "get_prompt",
            category: "prompt",
            description: "Browse and retrieve prompt templates. nn Actions:n - list_categories: Show all prompt categoriesn - list_prompts: List all prompts (optionally filt...",
            schema: build_schema::<prompt::GetPromptArgs>(),
        },
        // REASONER (1 tool)
        ToolMetadata {
            name: "sequential_thinking_reasoner",
            category: "reasoner",
            description: "Advanced reasoning tool with multiple strategies (Beam Search, MCTS). Processes thoughts step-by-step, supports branching and revision, and tracks best reasoning paths. Use for complex problem-solving that requires exploration of multiple solution approaches.\n\nStrategies:\n- beam_search: Breadth-first exploration (default)\n- mcts: Monte Carlo Tree Search with UCB1\n- mcts_002_alpha: High exploration MCTS variant\n- mcts_002alt_alpha: Length-rewarding MCTS variant\n\nOptional VoyageAI Embedding Integration: Set VOYAGE_API_KEY environment variable to enable semantic coherence scoring.",
            schema: build_schema::<reasoning::ReasonerArgs>(),
        },
        // SEQUENTIAL-THINKING (1 tool)
        ToolMetadata {
            name: "sequential_thinking",
            category: "sequential_thinking",
            description: "A detailed tool for dynamic and reflective problem-solving through thoughts. This tool helps analyze problems through a flexible thinking process that can adapt and evolve. Each thought can build on, question, or revise previous insights as understanding deepens.",
            schema: build_schema::<reasoning::SequentialThinkingArgs>(),
        },
    ]
}
