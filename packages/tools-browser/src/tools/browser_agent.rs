use crate::agent::{Agent, AgentHistoryList};
use crate::agent::prompts::{SystemPrompt, AgentMessagePrompt};
use crate::manager::BrowserManager;
use crate::utils::AgentState;
use kodegen_mcp_client::KodegenClient;
use kodegen_mcp_tool::{Tool, error::McpError};
use rmcp::model::{PromptArgument, PromptMessage, PromptMessageContent, PromptMessageRole};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserAgentArgs {
    /// Task description for the agent to accomplish
    pub task: String,
    
    /// Optional additional context or hints
    #[serde(default)]
    pub additional_info: Option<String>,
    
    /// Optional initial URL to navigate to before starting
    #[serde(default)]
    pub start_url: Option<String>,
    
    /// Maximum steps agent can take (default: 10)
    #[serde(default = "default_max_steps")]
    pub max_steps: u32,
    
    /// Maximum actions per step (default: 3)
    #[serde(default = "default_max_actions")]
    pub max_actions_per_step: u32,
    
    /// LLM temperature for action generation (default: 0.7)
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    
    /// Max tokens per LLM call (default: 2048)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u64,
    
    /// Vision model timeout in seconds (default: 60s)
    /// Vision analysis is typically fast, but allow time for model loading
    #[serde(default = "default_vision_timeout_secs")]
    pub vision_timeout_secs: u64,
    
    /// LLM generation timeout in seconds (default: 120s)
    /// Allow time for complex reasoning and high token generation
    #[serde(default = "default_llm_timeout_secs")]
    pub llm_timeout_secs: u64,
}

fn default_max_steps() -> u32 { 10 }
fn default_max_actions() -> u32 { 3 }
fn default_temperature() -> f64 { 0.7 }
fn default_max_tokens() -> u64 { 2048 }
fn default_vision_timeout_secs() -> u64 { 60 }
fn default_llm_timeout_secs() -> u64 { 120 }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserAgentPromptArgs {}

#[derive(Clone)]
pub struct BrowserAgentTool {
    _browser_manager: Arc<BrowserManager>,
    mcp_client: Arc<KodegenClient>,
}

impl BrowserAgentTool {
    pub fn new(
        browser_manager: Arc<BrowserManager>,
        mcp_client: Arc<KodegenClient>,
    ) -> Self {
        Self {
            _browser_manager: browser_manager,
            mcp_client,
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
        false  // Agent modifies browser state
    }

    fn open_world() -> bool {
        true  // Agent can navigate to any URL
    }

    async fn execute(&self, args: Self::Args) -> Result<Value, McpError> {
        // Navigate to start URL if provided (BEFORE creating agent)
        if let Some(url) = &args.start_url {
            self.mcp_client
                .call_tool("browser_navigate", json!({
                    "url": url,
                    "timeout_ms": 30000
                }))
                .await
                .map_err(|e| McpError::Other(anyhow::anyhow!(
                    "Failed to navigate to start URL: {}", e
                )))?;
        }

        // Create agent with all required parameters
        let system_prompt = SystemPrompt::new();
        let agent_prompt = AgentMessagePrompt::new();
        let agent_state = Arc::new(Mutex::new(AgentState::new()));
        
        let agent = Agent::new(
            &args.task,
            &args.additional_info.as_deref().unwrap_or(""),
            self.mcp_client.clone(),
            system_prompt,
            agent_prompt,
            args.max_actions_per_step as usize,
            agent_state,
            args.temperature,
            args.max_tokens,
            args.vision_timeout_secs,
            args.llm_timeout_secs,
        ).map_err(|e| McpError::Other(anyhow::anyhow!(
            "Failed to create agent: {}", e
        )))?;

        // Execute agent task
        let history = agent
            .run(args.max_steps as usize)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!(
                "Agent execution failed: {}", e
            )))?;

        // Build response with execution summary
        let steps_taken = history.steps.len();
        let is_complete = history.is_complete();
        let final_result = history.final_result().unwrap_or_else(|| 
            format!("Agent stopped after {} steps (incomplete)", steps_taken)
        );

        // Extract action summaries from history
        let actions: Vec<Value> = history.steps.iter().map(|step| {
            json!({
                "step": step.step,
                "timestamp": step.timestamp.to_rfc3339(),
                "actions": step.output.action.iter().map(|a| &a.action).collect::<Vec<_>>(),
                "summary": step.output.current_state.summary,
                "complete": step.is_complete,
            })
        }).collect();

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
                     {\\\"task\\\": \\\"Fill contact form with name='John' email='john@example.com'\\\", \
                       \\\"start_url\\\": \\\"https://example.com/contact\\\", \
                       \\\"max_steps\\\": 5, \
                       \\\"temperature\\\": 0.5}\\n\\n\
                     The agent will navigate, click, type, scroll, and extract content autonomously."
                ),
            },
        ])
    }
}
