//! Tools for managing Claude agent sessions
//!
//! Provides MCP tools for spawning, managing, and interacting with Claude agent sessions.

mod list_claude_agents;
mod read_claude_agent_output;
mod send_claude_agent_prompt;
mod spawn_claude_agent;
mod terminate_claude_agent_session;

pub use list_claude_agents::ListClaudeAgentsTool;
pub use read_claude_agent_output::ReadClaudeAgentOutputTool;
pub use send_claude_agent_prompt::SendClaudeAgentPromptTool;
pub use spawn_claude_agent::SpawnClaudeAgentTool;
pub use terminate_claude_agent_session::TerminateClaudeAgentSessionTool;
