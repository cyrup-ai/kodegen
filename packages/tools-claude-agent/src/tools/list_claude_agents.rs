use crate::manager::AgentManager;
use kodegen_mcp_schema::claude_agent::{ListClaudeAgentsArgs, ListClaudeAgentsPromptArgs};
use kodegen_mcp_tool::Tool;
use rmcp::model::{PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::Value;
use std::sync::Arc;

// ============================================================================
// ARGS STRUCTS - Imported from kodegen_mcp_schema::claude_agent
// ============================================================================

// ============================================================================
// TOOL STRUCT
// ============================================================================

/// MCP tool for listing all active and completed Claude agent sessions
#[derive(Clone)]
pub struct ListClaudeAgentsTool {
    agent_manager: Arc<AgentManager>,
}

impl ListClaudeAgentsTool {
    /// Create a new list agents tool with required dependencies
    #[must_use]
    pub fn new(agent_manager: Arc<AgentManager>) -> Self {
        Self { agent_manager }
    }
}

// ============================================================================
// TOOL TRAIT IMPLEMENTATION
// ============================================================================

impl Tool for ListClaudeAgentsTool {
    type Args = ListClaudeAgentsArgs;
    type PromptArgs = ListClaudeAgentsPromptArgs;

    fn name() -> &'static str {
        "list_claude_agents"
    }

    fn description() -> &'static str {
        "List all active and completed agent sessions with status and output preview. \
         Shows working indicator (true if actively processing), turn count, runtime, \
         message count, and last N lines of output for quick overview. \
         Agents are sorted with working agents first, then by most recent activity."
    }

    fn read_only() -> bool {
        true
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        true
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, kodegen_mcp_tool::error::McpError> {
        let response = self
            .agent_manager
            .list_sessions(args.include_completed, args.last_output_lines)
            .await
            .map_err(|e| kodegen_mcp_tool::error::McpError::Other(e.into()))?;

        serde_json::to_value(response)
            .map_err(|e| kodegen_mcp_tool::error::McpError::Other(e.into()))
    }

    fn prompt_arguments() -> Vec<rmcp::model::PromptArgument> {
        vec![]
    }

    async fn prompt(
        &self,
        _args: Self::PromptArgs,
    ) -> Result<Vec<PromptMessage>, kodegen_mcp_tool::error::McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::Text {
                text: r#"# list_claude_agents

List all active and completed agent sessions with status overview and output preview.

## Example: List all agents
```json
{
  "include_completed": true,
  "last_output_lines": 3
}
```

## Example: List only active agents
```json
{
  "include_completed": false
}
```

## Response
Returns array of agents sorted by:
1. Working agents first (working=true)
2. Then by most recent activity

Each agent includes:
- Session ID and label
- Working status (true = actively processing)
- Turn count and max_turns
- Runtime and message count
- Completion status and time
- Last N lines of output for preview

## Use Cases
- Monitor multiple parallel agents
- Check which agents are still working
- View progress across all tasks
- Identify stuck or completed agents
- Get quick output preview without full read"#
                    .to_string(),
            },
        }])
    }
}
