//! Candle agent (memory tools) metadata

use kodegen_mcp_schema::claude_agent::{
    MEMORY_MEMORIZE, MEMORY_CHECK_MEMORIZE_STATUS, MEMORY_RECALL, MEMORY_LIST_LIBRARIES,
    MemorizeArgs, CheckMemorizeStatusArgs, RecallArgs, ListMemoryLibrariesArgs,
};
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn candle_agent_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: MEMORY_MEMORIZE,
            category: "candle_agent",
            description: "Store content in a named memory library with automatic embedding generation. The memory will be tagged with the library name and can be retrieved later using recall(). Each library is a separate namespace for organizing memories.",
            schema: build_schema::<MemorizeArgs>(),
        },
        ToolMetadata {
            name: MEMORY_CHECK_MEMORIZE_STATUS,
            category: "candle_agent",
            description: "Check the status of an async memorize operation started with memorize().\n\nReturns current status, progress information, and memory_id when complete.\n\nStatus values:\n- IN_PROGRESS: Task is still running (loading content, generating embeddings, storing)\n- COMPLETED: Task finished successfully (memory_id available)\n- FAILED: Task failed (error message available)\n\nPoll this repeatedly (with delays) until status is COMPLETED or FAILED.\nProgress includes current stage (Loading content, Generating embeddings, Storing in database)\nand file counts for multi-file operations.",
            schema: build_schema::<CheckMemorizeStatusArgs>(),
        },
        ToolMetadata {
            name: MEMORY_RECALL,
            category: "candle_agent",
            description: "Retrieve relevant memories from a library using semantic search. Searches for content similar to the provided context and returns the most relevant results. Uses vector similarity (cosine) to find semantically related memories.",
            schema: build_schema::<RecallArgs>(),
        },
        ToolMetadata {
            name: MEMORY_LIST_LIBRARIES,
            category: "candle_agent",
            description: "List all unique memory library names that have been created. Returns a list of all libraries that contain at least one memory. Use this to discover what libraries are available for recall.",
            schema: build_schema::<ListMemoryLibrariesArgs>(),
        },
    ]
}
