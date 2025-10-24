use crate::manager::AgentManager;
use kodegen_mcp_schema::claude_agent::{SendClaudeAgentPromptArgs, SendClaudeAgentPromptPromptArgs};
use kodegen_mcp_tool::Tool;
use rmcp::model::{PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::sync::Arc;

// Import resolve method extension for schema's PromptInput type
use crate::types::prompt_input;

// ============================================================================
// ARGS STRUCTS - Imported from kodegen_mcp_schema::claude_agent
// ============================================================================

// ============================================================================
// TOOL STRUCT
// ============================================================================

/// MCP tool for sending prompts to running Claude agent sessions
#[derive(Clone)]
pub struct SendClaudeAgentPromptTool {
    agent_manager: Arc<AgentManager>,
    prompt_manager: Arc<kodegen_tools_prompt::PromptManager>,
}

impl SendClaudeAgentPromptTool {
    /// Create a new send prompt tool with required dependencies
    #[must_use]
    pub fn new(
        agent_manager: Arc<AgentManager>,
        prompt_manager: Arc<kodegen_tools_prompt::PromptManager>,
    ) -> Self {
        Self {
            agent_manager,
            prompt_manager,
        }
    }
}

// ============================================================================
// TOOL TRAIT IMPLEMENTATION
// ============================================================================

impl Tool for SendClaudeAgentPromptTool {
    type Args = SendClaudeAgentPromptArgs;
    type PromptArgs = SendClaudeAgentPromptPromptArgs;

    fn name() -> &'static str {
        "send_claude_agent_prompt"
    }

    fn description() -> &'static str {
        "Send a follow-up prompt to an active agent session. Continues the conversation \
         with new instructions or questions. Use read_claude_agent_output to poll for the \
         agent's response. Cannot send to completed sessions or sessions at max_turns."
    }

    fn read_only() -> bool {
        false
    }

    fn destructive() -> bool {
        false
    }

    fn idempotent() -> bool {
        false
    }

    fn open_world() -> bool {
        true
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, kodegen_mcp_tool::error::McpError> {
        // Resolve prompt (render template if needed)
        let resolved_prompt = prompt_input::resolve_schema_prompt(&args.prompt, &self.prompt_manager)
            .await
            .map_err(|e| kodegen_mcp_tool::error::McpError::Other(e.into()))?;

        self.agent_manager
            .send_message(&args.session_id, &resolved_prompt)
            .await
            .map_err(|e| kodegen_mcp_tool::error::McpError::Other(e.into()))?;

        let working = self
            .agent_manager
            .is_working(&args.session_id)
            .await
            .map_err(|e| kodegen_mcp_tool::error::McpError::Other(e.into()))?;

        let info = self
            .agent_manager
            .get_session_info(&args.session_id)
            .await
            .map_err(|e| kodegen_mcp_tool::error::McpError::Other(e.into()))?;

        Ok(json!({
            "session_id": args.session_id,
            "success": true,
            "turn_count": info.turn_count,
            "working": working
        }))
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
                text: r#"# send_claude_agent_prompt

Send a follow-up prompt to continue an active agent session's conversation.

## Example: Send follow-up with plain string
```json
{
  "session_id": "uuid-abc-123",
  "prompt": {
    "type": "string",
    "value": "Now fix those issues you found"
  }
}
```

## Example: Send follow-up with template
```json
{
  "session_id": "uuid-abc-123",
  "prompt": {
    "type": "template",
    "value": {
      "name": "follow_up_fix",
      "parameters": {
        "issue_id": "SEC-001",
        "approach": "refactor"
      }
    }
  }
}
```

## Response
Returns success status, updated turn count, and working indicator.

## Important Notes
- Cannot send to completed sessions (is_complete=true)
- Cannot send if at max_turns
- Agent begins processing immediately
- Use `read_claude_agent_output` to poll for response

## Workflow
1. Send prompt with this tool
2. Agent processes (working=true)
3. Poll with `read_claude_agent_output` for response
4. Repeat as needed"#
                    .to_string(),
            },
        }])
    }
}
