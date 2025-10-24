use crate::manager::AgentManager;
use kodegen_mcp_schema::claude_agent::{TerminateClaudeAgentSessionArgs, TerminateClaudeAgentSessionPromptArgs};
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

/// MCP tool for terminating running Claude agent sessions
#[derive(Clone)]
pub struct TerminateClaudeAgentSessionTool {
    agent_manager: Arc<AgentManager>,
}

impl TerminateClaudeAgentSessionTool {
    /// Create a new terminate session tool with required dependencies
    #[must_use]
    pub fn new(agent_manager: Arc<AgentManager>) -> Self {
        Self { agent_manager }
    }
}

// ============================================================================
// TOOL TRAIT IMPLEMENTATION
// ============================================================================

impl Tool for TerminateClaudeAgentSessionTool {
    type Args = TerminateClaudeAgentSessionArgs;
    type PromptArgs = TerminateClaudeAgentSessionPromptArgs;

    fn name() -> &'static str {
        "terminate_claude_agent_session"
    }

    fn description() -> &'static str {
        "Gracefully terminate an agent session. Closes the ClaudeSDKClient connection, \
         returns final statistics (turn count, message count, runtime), and moves the \
         session to completed state. Completed sessions are retained for 1 minute for \
         final reads before cleanup."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        true
    }

    fn idempotent() -> bool {
        false
    }

    fn open_world() -> bool {
        false
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, kodegen_mcp_tool::error::McpError> {
        let response = self
            .agent_manager
            .terminate_session(&args.session_id)
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
                text: r#"# terminate_claude_agent_session

Gracefully terminate an agent session and return final statistics.

## Example: Terminate session
```json
{
  "session_id": "uuid-abc-123"
}
```

## Response
Returns final statistics including:
- `final_turn_count`: Total turns completed
- `total_messages`: Total messages collected
- `runtime_ms`: Total runtime in milliseconds

## What Happens
1. Closes ClaudeSDKClient connection
2. Moves session to completed state
3. Session retained for 1 minute for final reads
4. Automatic cleanup after retention period

## When to Use
- Task completed successfully
- Need to free resources
- Agent reached unsatisfactory state
- Shutting down parent process"#
                    .to_string(),
            },
        }])
    }
}
