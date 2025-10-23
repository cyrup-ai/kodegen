use kodegen_mcp_tool::Tool;
use crate::manager::AgentManager;
use rmcp::model::{PromptMessage, PromptMessageRole, PromptMessageContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

// ============================================================================
// ARGS STRUCTS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadClaudeAgentOutputArgs {
    /// Session ID to read from
    pub session_id: String,
    
    /// Offset for pagination (0=start, negative=tail from end)
    #[serde(default)]
    pub offset: i64,
    
    /// Max messages to return (default: 50)
    #[serde(default = "default_length")]
    pub length: usize,
}

fn default_length() -> usize { 50 }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadClaudeAgentOutputPromptArgs {}

// ============================================================================
// TOOL STRUCT
// ============================================================================

/// MCP tool for reading paginated output from Claude agent sessions
#[derive(Clone)]
pub struct ReadClaudeAgentOutputTool {
    agent_manager: Arc<AgentManager>,
}

impl ReadClaudeAgentOutputTool {
    /// Create a new read output tool with required dependencies
    #[must_use] 
    pub fn new(agent_manager: Arc<AgentManager>) -> Self {
        Self { agent_manager }
    }
}

// ============================================================================
// TOOL TRAIT IMPLEMENTATION
// ============================================================================

impl Tool for ReadClaudeAgentOutputTool {
    type Args = ReadClaudeAgentOutputArgs;
    type PromptArgs = ReadClaudeAgentOutputPromptArgs;

    fn name() -> &'static str {
        "read_claude_agent_output"
    }

    fn description() -> &'static str {
        "Read paginated output from an agent session. Returns messages with working indicator. \
         Use offset/length for pagination (offset=0 for start, negative for tail). \
         Includes working status (true if actively processing, false if idle/complete). \
         Non-destructive read - messages persist in buffer."
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
        let response = self.agent_manager
            .get_output(&args.session_id, args.offset, args.length)
            .await
            .map_err(|e| kodegen_mcp_tool::error::McpError::Other(e.into()))?;
        
        serde_json::to_value(response)
            .map_err(|e| kodegen_mcp_tool::error::McpError::Other(e.into()))
    }

    fn prompt_arguments() -> Vec<rmcp::model::PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, kodegen_mcp_tool::error::McpError> {
        Ok(vec![PromptMessage {
            role: PromptMessageRole::User,
            content: PromptMessageContent::Text {
                text: r#"# read_claude_agent_output

Read paginated output from an agent session. Returns messages with working indicator (true = actively processing).

## Example: Read latest messages
```json
{
  "session_id": "uuid-abc-123"
}
```

## Example: Read first 100 messages
```json
{
  "session_id": "uuid-abc-123",
  "offset": 0,
  "length": 100
}
```

## Example: Read last 20 messages (tail)
```json
{
  "session_id": "uuid-abc-123",
  "offset": -20
}
```

## Response
Returns messages array with full content, working status, turn count, completion status, and pagination info (has_more).

## Key Fields
- `working`: true if agent actively processing, false if idle/complete
- `is_complete`: true if conversation finished (max_turns or Result message received)
- `has_more`: true if more messages available for pagination"#.to_string(),
            },
        }])
    }
}
