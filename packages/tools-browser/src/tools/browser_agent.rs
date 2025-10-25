use crate::agent::Agent;
use crate::agent::prompts::{AgentMessagePrompt, SystemPrompt};
use crate::manager::BrowserManager;
use crate::utils::AgentState;
use kodegen_mcp_schema::browser::{BrowserAgentArgs, BrowserAgentPromptArgs};
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct BrowserAgentTool {
    _browser_manager: Arc<BrowserManager>,
    server_url: String,
}

impl BrowserAgentTool {
    pub fn new(browser_manager: Arc<BrowserManager>, server_url: String) -> Self {
        Self {
            _browser_manager: browser_manager,
            server_url,
        }
    }
}

impl Tool for BrowserAgentTool {
    type Args = BrowserAgentArgs;
    type PromptArgs = BrowserAgentPromptArgs;

    fn name() -> &'static str {
        "browser_agent"
    }

    fn description() -> &'static str {
        "Autonomous browser agent that executes multi-step tasks using AI reasoning.\\n\\n\
         The agent can navigate websites, interact with forms, extract information,\\n\
         and complete complex workflows across multiple pages.\\n\\n\
         Example: browser_agent({\\\"task\\\": \\\"Find latest Rust version and save to file\\\", \\\"start_url\\\": \\\"https://rust-lang.org\\\", \\\"max_steps\\\": 8})"
    }

    fn read_only() -> bool {
        false // Agent modifies browser state
    }

    fn open_world() -> bool {
        true // Agent can navigate to any URL
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Create loopback MCP client (connects to same server)
        // By the time this tool executes, the server is fully running
        let (mcp_client, _connection) = kodegen_mcp_client::create_sse_client(&self.server_url)
            .await
            .map_err(|e| {
                McpError::Other(anyhow::anyhow!(
                    "Failed to create loopback client to {}: {}. \
                 Ensure SSE server is running and accessible.",
                    self.server_url,
                    e
                ))
            })?;

        // Navigate to start URL if provided (BEFORE creating agent)
        if let Some(url) = &args.start_url {
            mcp_client
                .call_tool(
                    "browser_navigate",
                    json!({
                        "url": url,
                        "timeout_ms": 30000
                    }),
                )
                .await
                .map_err(|e| {
                    McpError::Other(anyhow::anyhow!("Failed to navigate to start URL: {}", e))
                })?;
        }

        // Create agent with all required parameters
        let prompts = crate::agent::PromptConfig {
            system_prompt: SystemPrompt::new(),
            agent_prompt: AgentMessagePrompt::new(),
        };
        let agent_state = Arc::new(Mutex::new(AgentState::new()));

        let config = crate::agent::AgentConfig {
            temperature: args.temperature,
            max_tokens: args.max_tokens,
            vision_timeout_secs: args.vision_timeout_secs,
            llm_timeout_secs: args.llm_timeout_secs,
        };

        let agent = Agent::new(
            &args.task,
            args.additional_info.as_deref().unwrap_or(""),
            Arc::new(mcp_client),
            prompts,
            args.max_actions_per_step as usize,
            agent_state,
            config,
        )
        .map_err(|e| McpError::Other(anyhow::anyhow!("Failed to create agent: {}", e)))?;

        // Execute agent task
        let history = agent
            .run(args.max_steps as usize)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Agent execution failed: {}", e)))?;

        // Build response with execution summary
        let steps_taken = history.steps.len();
        let is_complete = history.is_complete();
        let final_result = history
            .final_result()
            .unwrap_or_else(|| format!("Agent stopped after {} steps (incomplete)", steps_taken));

        // Extract action summaries from history
        let actions: Vec<Value> = history
            .steps
            .iter()
            .map(|step| {
                json!({
                    "step": step.step,
                    "timestamp": step.timestamp.to_rfc3339(),
                    "actions": step.output.action.iter().map(|a| &a.action).collect::<Vec<_>>(),
                    "summary": step.output.current_state.summary,
                    "complete": step.is_complete,
                })
            })
            .collect();

        Ok(json!({
            "success": is_complete,
            "steps_taken": steps_taken,
            "max_steps": args.max_steps,
            "final_result": final_result,
            "task": args.task,
            "actions": actions,
        }))
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![
            PromptMessage {
                role: PromptMessageRole::User,
                content: PromptMessageContent::text("How do I use the browser agent?"),
            },
            PromptMessage {
                role: PromptMessageRole::Assistant,
                content: PromptMessageContent::text(
                    "Use browser_agent to automate multi-step browser tasks with AI reasoning.\\n\\n\
                     Research example:\\n\
                     {\\\"task\\\": \\\"Find latest Rust release and save version to notes.txt\\\", \
                       \\\"start_url\\\": \\\"https://www.rust-lang.org/\\\", \
                       \\\"max_steps\\\": 8}\\n\\n\
                     Form filling example:\\n\
                     {\\\"task\\\": \\\"Fill contact form with name='John' email='john@test.local'\\\", \
                       \\\"start_url\\\": \\\"https://httpbin.org/forms/post\\\", \
                       \\\"max_steps\\\": 5, \
                       \\\"temperature\\\": 0.5}\\n\\n\
                     The agent will navigate, click, type, scroll, and extract content autonomously.",
                ),
            },
        ])
    }
}
