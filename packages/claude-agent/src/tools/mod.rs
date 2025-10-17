//! Tools for managing Claude agent sessions
//!
//! Provides MCP tools for spawning, managing, and interacting with Claude agent sessions.

mod spawn_claude_agent;
mod read_claude_agent_output;
mod send_claude_agent_prompt;
mod terminate_claude_agent_session;
mod list_claude_agents;

pub use spawn_claude_agent::SpawnClaudeAgentTool;
pub use read_claude_agent_output::ReadClaudeAgentOutputTool;
pub use send_claude_agent_prompt::SendClaudeAgentPromptTool;
pub use terminate_claude_agent_session::TerminateClaudeAgentSessionTool;
pub use list_claude_agents::ListClaudeAgentsTool;
