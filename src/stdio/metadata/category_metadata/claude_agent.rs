//! Claude agent tool metadata

use kodegen_mcp_schema::claude_agent;
use crate::stdio::metadata::types::{build_schema, ToolMetadata};

pub fn claude_agent_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            name: claude_agent::CLAUDE_AGENT,
            category: "claude_agent",
            description: "Unified Claude agent interface with action-based dispatch (SPAWN/SEND/READ/LIST/KILL). Each connection gets independent agent numbering (agent:0, agent:1, agent:2). Supports timeout with background continuation.\n\nActions:\n• SPAWN: Create new agent session with initial prompt\n• SEND: Send additional prompt to existing agent\n• READ: Read current agent output\n• LIST: List all agents for this connection\n• KILL: Terminate agent and cleanup",
            schema: build_schema::<claude_agent::ClaudeAgentArgs>(),
        },
    ]
}
