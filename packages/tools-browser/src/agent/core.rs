use std::sync::Arc;

// NEW - Workspace LLM and Vision infrastructure
use kodegen_candle_agent::prelude::*;
use tokio_stream::StreamExt; // For stream.next().await

use base64::Engine; // For base64 decode in async context
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info, warn};

// MCP client for hot path integration
use kodegen_mcp_client::KodegenClient;

use crate::agent::{
    ActionModel, ActionResult, AgentError, AgentHistoryList, AgentLLMResponse, AgentOutput,
    AgentResult, BrowserExtractTextResponse, BrowserScreenshotResponse,
    prompts::{AgentMessagePrompt, SystemPrompt},
};
use crate::utils::AgentState;

/// Configuration parameters for agent behavior
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub temperature: f64,
    pub max_tokens: u64,
    pub vision_timeout_secs: u64,
    pub llm_timeout_secs: u64,
}

/// Prompt configuration for agent
#[derive(Debug, Clone)]
pub struct PromptConfig {
    pub system_prompt: SystemPrompt,
    pub agent_prompt: AgentMessagePrompt,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 4096,
            vision_timeout_secs: 30,
            llm_timeout_secs: 120,
        }
    }
}

/// Shared agent state and processing logic (can be Arc-cloned)
struct AgentInner {
    task: String,
    add_infos: String,
    mcp_client: Arc<KodegenClient>,
    system_prompt: SystemPrompt,
    agent_prompt: AgentMessagePrompt,
    max_actions_per_step: usize,
    agent_state: Arc<Mutex<AgentState>>,
    temperature: f64,
    max_tokens: u64,
    vision_timeout_secs: u64,
    llm_timeout_secs: u64,
}

/// Agent handle for controlling async actor (NOT Clone)
pub struct Agent {
    inner: Arc<AgentInner>,
    command_channel: mpsc::Sender<AgentCommand>,
    response_channel: Mutex<mpsc::Receiver<AgentResponse>>,

    /// Background processor task handle
    ///
    /// Stores the JoinHandle for the spawned agent processor task.
    /// This ensures the task is tracked and can be awaited for graceful shutdown.
    /// Following the pattern from kodegen_tools_citescrape::CrawlSession.
    #[allow(dead_code)]
    processor_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Agent command enum for internal message passing
enum AgentCommand {
    RunStep,
    Stop,
}

/// Agent response enum for internal message passing
#[derive(Debug)]
enum AgentResponse {
    StepComplete(AgentOutput),
    Error(String),
    Stopped,
}

///  agent implementation
impl Agent {
    /// Create a new agent instance
    pub fn new(
        task: &str,
        add_infos: &str,
        mcp_client: Arc<KodegenClient>,
        prompts: PromptConfig,
        max_actions_per_step: usize,
        agent_state: Arc<Mutex<AgentState>>,
        config: AgentConfig,
    ) -> AgentResult<Self> {
        // Create channels for command passing
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (resp_tx, resp_rx) = mpsc::channel(32);

        // Create shared inner state (Arc-wrapped)
        let inner = Arc::new(AgentInner {
            task: task.to_string(),
            add_infos: add_infos.to_string(),
            mcp_client,
            system_prompt: prompts.system_prompt,
            agent_prompt: prompts.agent_prompt,
            max_actions_per_step,
            agent_state,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            vision_timeout_secs: config.vision_timeout_secs,
            llm_timeout_secs: config.llm_timeout_secs,
        });

        // Spawn processor with Arc-cloned inner and store handle
        let processor_handle = Self::spawn_agent_processor(Arc::clone(&inner), cmd_rx, resp_tx);

        // Return handle with unique receiver ownership
        Ok(Self {
            inner,
            command_channel: cmd_tx,
            response_channel: Mutex::new(resp_rx),
            processor_handle: Some(processor_handle),
        })
    }

    /// Run the agent to perform a task with a maximum number of steps
    pub async fn run(&self, max_steps: usize) -> AgentResult<AgentHistoryList> {
        let mut history = AgentHistoryList::new();

        for step in 0..max_steps {
            debug!("Running agent step {}/{}", step + 1, max_steps);

            // Check if processor was stopped externally
            if !self.is_running() {
                info!("Agent processor stopped externally, exiting run loop");
                break;
            }

            // Check if stop was requested via AgentState
            if self.is_stop_requested().await {
                info!("Agent run stopped as requested");
                break;
            }

            // Run a single step
            match self.run_step().await {
                Ok(output) => {
                    // Record step output
                    let is_done = output
                        .action
                        .iter()
                        .any(|a| a.action.eq_ignore_ascii_case("done"));
                    history.add_step_with_completion(output.clone(), is_done);

                    // Check if agent considers task complete
                    // Protocol: done if any action is "done" or "Done"
                    if is_done {
                        info!("Agent completed task in {} steps", step + 1);
                        break;
                    }
                }
                Err(e) => {
                    error!("Agent step error: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(history)
    }

    /// Run a single agent step
    async fn run_step(&self) -> AgentResult<AgentOutput> {
        // Send command to agent processor
        self.command_channel
            .send(AgentCommand::RunStep)
            .await
            .map_err(|_| AgentError::ChannelClosed("Command channel closed".into()))?;

        // Wait for response (lock mutex to access receiver)
        let mut receiver = self.response_channel.lock().await;
        match receiver.recv().await {
            Some(AgentResponse::StepComplete(output)) => Ok(output),
            Some(AgentResponse::Error(msg)) => Err(AgentError::StepFailed(msg)),
            Some(AgentResponse::Stopped) => Err(AgentError::Stopped),
            None => Err(AgentError::ChannelClosed("Response channel closed".into())),
        }
    }

    /// Check if agent stop was requested
    async fn is_stop_requested(&self) -> bool {
        let agent_state = self.inner.agent_state.lock().await;
        agent_state.is_stop_requested()
    }

    /// Gracefully shut down the agent processor
    ///
    /// Sends Stop command and waits for processor to confirm shutdown.
    /// Returns when processor has fully stopped and cleaned up resources.
    ///
    /// # Errors
    /// - `AgentError::ChannelClosed`: Command channel already closed (processor dead)
    /// - `AgentError::UnexpectedError`: Processor didn't respond within timeout
    /// - `AgentError::UnexpectedError`: Processor sent unexpected response
    pub async fn stop(&self) -> AgentResult<()> {
        debug!("Stopping agent processor");

        // Send stop command
        self.command_channel
            .send(AgentCommand::Stop)
            .await
            .map_err(|_| {
                AgentError::ChannelClosed(
                    "Cannot stop agent: command channel already closed".into(),
                )
            })?;

        // Wait for Stopped confirmation with timeout
        // Pattern adapted from run_step() (lines 171-179)
        let mut receiver = self.response_channel.lock().await;

        match tokio::time::timeout(
            Duration::from_secs(5), // Processor should stop quickly
            receiver.recv(),
        )
        .await
        {
            Ok(Some(AgentResponse::Stopped)) => {
                info!("Agent processor stopped gracefully");
                Ok(())
            }
            Ok(Some(other)) => {
                warn!("Expected Stopped response, got: {:?}", other);
                Err(AgentError::UnexpectedError(
                    "Agent processor sent unexpected response to Stop command".into(),
                ))
            }
            Ok(None) => {
                warn!("Agent response channel closed during stop");
                // Channel closed = processor dead = effectively stopped
                Ok(())
            }
            Err(_) => {
                error!("Agent processor did not respond to Stop within 5 seconds");
                Err(AgentError::UnexpectedError(
                    "Agent processor stop timeout - processor may be stuck".into(),
                ))
            }
        }
    }

    /// Check if agent processor is still running
    ///
    /// Returns `true` if the processor task is active and accepting commands.
    /// Returns `false` if the processor has stopped (command channel closed).
    ///
    /// This is useful for:
    /// - Checking processor state before sending commands
    /// - Polling for processor completion
    /// - Debugging processor lifecycle
    pub fn is_running(&self) -> bool {
        // Processor is running if command channel is still open
        // When processor exits, it drops cmd_rx which closes the channel
        !self.command_channel.is_closed()
    }

    /// Spawn the agent processor task
    fn spawn_agent_processor(
        inner: Arc<AgentInner>,
        mut cmd_rx: mpsc::Receiver<AgentCommand>,
        resp_tx: mpsc::Sender<AgentResponse>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    AgentCommand::RunStep => {
                        let result = inner.process_step().await;

                        // Map result to response
                        let response = match result {
                            Ok(output) => AgentResponse::StepComplete(output),
                            Err(e) => AgentResponse::Error(e.to_string()),
                        };

                        // Send response and only break if channel closed
                        if let Err(e) = resp_tx.send(response).await {
                            error!("Failed to send response: {}", e);
                            break;
                        }
                    }
                    AgentCommand::Stop => {
                        if let Err(e) = resp_tx.send(AgentResponse::Stopped).await {
                            error!("Failed to send stopped response: {}", e);
                        }
                        break;
                    }
                }
            }
            debug!("Agent processor shutting down cleanly");
        })
    }
}

/// Implementation of processing methods on AgentInner
impl AgentInner {
    /// Process a single agent step internally
    async fn process_step(&self) -> AgentResult<AgentOutput> {
        // Check if stop requested
        let agent_state = self.agent_state.lock().await;
        if agent_state.is_stop_requested() {
            return Err(AgentError::Stopped);
        }
        drop(agent_state);

        // Get current browser state (with screenshot)
        let mut browser_state = self.get_browser_state().await?;

        // Generate agent actions using CandleFluentAi LLM (with vision analysis if screenshot available)
        let llm_response = self.generate_actions_with_llm(&mut browser_state).await?;

        // Execute actions via MCP hot path
        let (_action_results, errors) = self.execute_actions(llm_response.action.clone()).await?;

        // Log errors if any
        if !errors.is_empty() {
            warn!("Action execution errors: {:?}", errors);
        }

        // Return output with LLM-generated state (no wasteful rebuilding!)
        Ok(AgentOutput {
            current_state: llm_response.current_state,
            action: llm_response.action,
        })
    }

    /// Get current browser state for LLM context (HOT PATH!)
    ///
    /// Fetches page content and optional screenshot via MCP tools.
    /// This provides the LLM with current browser context for action planning.
    ///
    /// Uses:
    /// - browser_extract_text: Get page text content
    /// - browser_screenshot: Get base64-encoded screenshot (optional)
    ///
    /// Returns BrowserStateWithScreenshot with text summary and screenshot.
    async fn get_browser_state(&self) -> AgentResult<BrowserStateWithScreenshot> {
        // Extract page content via MCP (HOT PATH!)
        let content = match self
            .mcp_client
            .call_tool("browser_extract_text", serde_json::json!({}))
            .await
        {
            Ok(result) => {
                // Parse text from tool response
                // browser_extract_text returns: {"success": true, "text": "...", "length": N, ...}
                result
                    .content
                    .first()
                    .and_then(|c| c.as_text())
                    .and_then(|t| {
                        serde_json::from_str::<BrowserExtractTextResponse>(&t.text)
                            .ok()
                            .map(|response| response.text)
                    })
                    .unwrap_or_else(|| {
                        warn!("Failed to parse browser_extract_text response, using empty content");
                        String::new()
                    })
            }
            Err(e) => {
                warn!("browser_extract_text failed: {}, using empty content", e);
                String::new()
            }
        };

        // Get screenshot via MCP and save to temp file (HOT PATH!)
        let screenshot_path = match self
            .mcp_client
            .call_tool("browser_screenshot", serde_json::json!({}))
            .await
        {
            Ok(result) => {
                // Parse base64 image from tool response
                // ⚠️ CRITICAL: browser_screenshot returns {"image": base64}, NOT {"base64": base64}!
                // See packages/tools-browser/src/tools/screenshot.rs:148-156
                let screenshot_base64 =
                    result
                        .content
                        .first()
                        .and_then(|c| c.as_text())
                        .and_then(|t| {
                            serde_json::from_str::<BrowserScreenshotResponse>(&t.text)
                                .ok()
                                .map(|response| response.image)
                        });

                // Save base64 to temp file for vision API
                if let Some(base64_data) = screenshot_base64 {
                    // ✅ FIX 1: Move CPU-intensive base64 decode to blocking thread pool
                    // This prevents the decode operation from blocking tokio worker threads
                    let decoded_bytes = tokio::task::spawn_blocking(move || {
                        base64::engine::general_purpose::STANDARD.decode(&base64_data)
                    })
                    .await // Wait for blocking task to complete (doesn't block thread!)
                    .map_err(|e| {
                        AgentError::UnexpectedError(format!("Base64 decode task failed: {}", e))
                    })? // Handle JoinError
                    .map_err(|e| {
                        AgentError::UnexpectedError(format!("Base64 decode failed: {}", e))
                    })?; // Handle decode error

                    // Create unique temp file path with nanosecond precision + PID
                    let temp_dir = std::env::temp_dir();
                    let duration = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| {
                            AgentError::BrowserError(format!("System time error: {}", e))
                        })?;

                    let filename = format!(
                        "browser_screenshot_{}_{:09}_{}.png",
                        duration.as_secs(),
                        duration.subsec_nanos(),
                        std::process::id()
                    );
                    let temp_path = temp_dir.join(filename);

                    // Example output: browser_screenshot_1735077890_123456789_42.png
                    //                 ^-seconds----^  ^-nanos----^  ^-PID-^

                    // ✅ FIX 2: Use async file write instead of blocking std::fs::write
                    // This allows other async tasks to progress during I/O
                    match tokio::fs::write(&temp_path, decoded_bytes).await {
                        Ok(_) => Some(temp_path.to_string_lossy().to_string()),
                        Err(e) => {
                            warn!("Failed to write screenshot to file: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            }
            Err(e) => {
                warn!(
                    "browser_screenshot failed: {}, continuing without screenshot",
                    e
                );
                None
            }
        };

        // Build state representation for LLM
        let state = format!(
            "Content Length: {} characters\nContent Sample: {}{}",
            content.len(),
            &content[0..content.len().min(500)],
            if content.len() > 500 { "..." } else { "" }
        );

        // Store state for recovery if needed
        let mut agent_state = self.agent_state.lock().await;
        agent_state.set_last_valid_state(state.clone());
        drop(agent_state);

        Ok(BrowserStateWithScreenshot {
            state,
            screenshot_path,
            visual_description: None, // Will be populated by format_browser_state_with_vision()
        })
    }

    /// Format browser state with vision-based screenshot analysis
    ///
    /// Uses CandleFluentAi::vision() to analyze screenshots and generate
    /// detailed visual descriptions of UI elements and layout.
    ///
    /// Populates browser_state.visual_description with the vision analysis result
    /// for potential caching/reuse.
    async fn format_browser_state_with_vision(
        &self,
        browser_state: &mut BrowserStateWithScreenshot,
    ) -> AgentResult<String> {
        use cyrup_sugars::prelude::MessageChunk;

        let mut state_description = format!("Current browser state:\n{}", browser_state.state);

        // Add vision-based screenshot analysis if available
        if let Some(screenshot_path) = &browser_state.screenshot_path {
            state_description.push_str("\n\nVisual Analysis:\n");

            // Check if we already have cached visual description
            let visual_desc = if let Some(ref cached) = browser_state.visual_description {
                debug!("Using cached visual description");
                cached.clone()
            } else {
                // Generate new vision analysis
                let vision_query = "Describe the visible UI elements, their layout, and any interactive components (buttons, links, forms, input fields, etc.) in detail.";

                // Wrap entire stream consumption in timeout
                let vision_timeout = Duration::from_secs(self.vision_timeout_secs);
                let result = timeout(vision_timeout, async {
                    // Vision responses typically 200-1000 tokens (~4KB conservative estimate)
                    // Pre-allocate to avoid reallocations during streaming
                    let mut description = String::with_capacity(4096);
                    let mut stream =
                        CandleFluentAi::vision().describe_image(screenshot_path, vision_query);

                    while let Some(chunk) = stream.next().await {
                        // Check for errors
                        if let Some(error) = chunk.error() {
                            return Err(format!("Vision analysis error: {}", error));
                        }

                        // Accumulate text
                        if !chunk.text.is_empty() {
                            description.push_str(&chunk.text);
                        }

                        // Check for completion
                        if chunk.is_final {
                            if let Some(stats) = &chunk.stats {
                                debug!(
                                    "Vision analysis: {} tokens generated",
                                    stats.tokens_generated
                                );
                            }
                            return Ok(description);
                        }
                    }
                    // Stream ended without is_final
                    Err("Vision stream ended without final chunk".to_string())
                })
                .await;

                match result {
                    Ok(Ok(desc)) => {
                        // Success: cache the description for potential reuse
                        browser_state.visual_description = Some(desc.clone());
                        desc
                    }
                    Ok(Err(e)) => {
                        // Stream error
                        warn!("Vision analysis failed: {}", e);
                        format!("[Vision analysis failed: {}]", e)
                    }
                    Err(_) => {
                        // Timeout
                        warn!(
                            "Vision analysis timed out after {}s",
                            self.vision_timeout_secs
                        );
                        format!(
                            "[Vision analysis timed out after {}s]",
                            self.vision_timeout_secs
                        )
                    }
                }
            };

            state_description.push_str(&visual_desc);
            state_description.push('\n');

            // Clean up temp screenshot file after vision analysis completes
            if let Err(e) = tokio::fs::remove_file(screenshot_path).await {
                warn!(
                    "Failed to cleanup screenshot file {}: {}",
                    screenshot_path, e
                );
            }
        }

        Ok(state_description)
    }

    /// Generate actions using CandleFluentAi LLM
    ///
    /// Combines system prompt, task description, and browser state into a query,
    /// then streams the LLM response and parses actions from it.
    async fn generate_actions_with_llm(
        &self,
        browser_state: &mut BrowserStateWithScreenshot,
    ) -> AgentResult<AgentLLMResponse> {
        // Build browser state message with vision analysis
        let browser_state_msg = self.format_browser_state_with_vision(browser_state).await?;

        // Build system prompt with available actions
        let actions_description = r##"Available Actions:
- go_to_url: Navigate to a URL (parameters: url)
- click_element: Click an element (parameters: selector OR index)
- input_text: Type text into an element (parameters: selector OR index, text)
- scroll: Scroll the page (parameters: direction ["up"|"down"|"left"|"right"], amount [pixels])
- extract_page_content: Extract page text content (no parameters)
- done: Mark task as complete (parameters: result [description of completion])

Parameter Notes:
- selector: CSS selector string (e.g., "#submit", ".button", "input[name='email']")
- index: Numeric index for data-mcp-index attributes (converted to selector automatically)
- Use selector for precision, index for LLM-generated element references

You must respond with valid JSON matching the AgentLLMResponse schema with an 'action' array."##;

        let system_prompt = format!(
            "{}\n\n{}\n\nYou are a browser automation agent. Analyze the browser state and generate appropriate actions.",
            self.system_prompt.build_prompt(),
            actions_description
        );

        // Build user query using AgentMessagePrompt (CRITICAL: integrates agent_prompt field)
        // This provides protocol-compliant instructions and proper context formatting
        let user_query =
            self.agent_prompt
                .build_message_prompt(&browser_state_msg, &self.task, &self.add_infos);

        // Stream LLM response with timeout protection
        let llm_timeout = Duration::from_secs(self.llm_timeout_secs);
        let full_response = match timeout(llm_timeout, async {
            // Pre-allocate based on max_tokens parameter
            // Average: ~4 bytes per token for English text
            let expected_bytes = (self.max_tokens as usize) * 4;
            let mut response = String::with_capacity(expected_bytes);
            let mut stream = CandleFluentAi::agent_role("browser-agent")
                .temperature(self.temperature)
                .max_tokens(self.max_tokens)
                .system_prompt(&system_prompt)
                .into_agent()
                .map_err(|e| AgentError::UnexpectedError(e.to_string()))?
                .chat(move |_conversation| {
                    let query = user_query.clone();
                    async move { CandleChatLoop::UserPrompt(query) }
                })
                .map_err(|e| AgentError::LlmError(e.to_string()))?;

            // Collect streaming response
            while let Some(chunk) = stream.next().await {
                match chunk {
                    CandleMessageChunk::Text(text) => {
                        response.push_str(&text);
                    }
                    CandleMessageChunk::Complete {
                        token_count,
                        elapsed_secs,
                        ..
                    } => {
                        if let (Some(tokens), Some(elapsed)) = (token_count, elapsed_secs) {
                            debug!("LLM generated {} tokens in {:.2}s", tokens, elapsed);
                        }
                        return Ok(response);
                    }
                    CandleMessageChunk::Error(err) => {
                        return Err(AgentError::LlmError(err.to_string()));
                    }
                    _ => {}
                }
            }
            // Stream ended without Complete chunk
            Err(AgentError::LlmError(
                "LLM stream ended without Complete chunk".into(),
            ))
        })
        .await
        {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(AgentError::LlmError(format!(
                    "LLM generation timed out after {}s",
                    self.llm_timeout_secs
                )));
            }
        };

        // Parse actions from JSON response
        let agent_response: AgentLLMResponse =
            serde_json::from_str(&full_response).map_err(|e| {
                AgentError::LlmError(format!(
                    "Failed to parse LLM response as JSON: {}. Response: {}",
                    e, full_response
                ))
            })?;

        // Limit the number of actions
        let limited_actions = if agent_response.action.len() > self.max_actions_per_step {
            warn!(
                "Agent generated {} actions, limiting to {}",
                agent_response.action.len(),
                self.max_actions_per_step
            );
            agent_response.action[0..self.max_actions_per_step].to_vec()
        } else {
            agent_response.action
        };

        Ok(AgentLLMResponse {
            current_state: agent_response.current_state,
            action: limited_actions,
        })
    }

    /// Execute actions by calling existing MCP tools (HOT PATH!)
    ///
    /// Maps agent protocol action names to MCP tool names and parameters.
    /// Each action is translated to an MCP call via self.mcp_client.call_tool().
    ///
    /// Action mapping (agent protocol → MCP tool):
    /// - go_to_url → browser_navigate
    /// - click_element → browser_click  
    /// - input_text → browser_type_text
    /// - scroll → browser_scroll
    /// - extract_page_content → browser_extract_text
    /// - done → (special case, no MCP call)
    ///
    async fn execute_actions(
        &self,
        actions: Vec<ActionModel>,
    ) -> AgentResult<(Vec<ActionResult>, Vec<String>)> {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for action in actions {
            // Map agent action names to MCP tool names (HOT PATH!)
            let (tool_name, tool_args) =
                match action.action.as_str() {
                    "go_to_url" => {
                        let url = action.parameters.get("url").ok_or_else(|| {
                            AgentError::StepFailed("Missing 'url' parameter".into())
                        })?;
                        (
                            "browser_navigate",
                            serde_json::json!({
                                "url": url,
                                "timeout_ms": 30000
                            }),
                        )
                    }
                    "click_element" => {
                        // Support both direct selector and index-based selector
                        // Converts index to [data-mcp-index="N"] selector
                        let selector = if let Some(selector) = action.parameters.get("selector") {
                            selector.clone()
                        } else if let Some(index) = action.parameters.get("index") {
                            // ✅ FIXED: Validate index is numeric before using in selector
                            let index_num = index.parse::<u64>().map_err(|_| {
                                AgentError::StepFailed(format!(
                                    "Invalid index parameter: must be numeric, got '{}'",
                                    index
                                ))
                            })?;
                            format!("[data-mcp-index=\"{}\"]", index_num)
                        } else {
                            return Err(AgentError::StepFailed(
                                "Missing 'selector' or 'index' parameter".into(),
                            ));
                        };
                        (
                            "browser_click",
                            serde_json::json!({
                                "selector": selector,
                                "timeout_ms": 5000
                            }),
                        )
                    }
                    "input_text" => {
                        // Support both direct selector and index-based selector
                        let selector = if let Some(selector) = action.parameters.get("selector") {
                            selector.clone()
                        } else if let Some(index) = action.parameters.get("index") {
                            // ✅ FIXED: Validate index is numeric before using in selector
                            let index_num = index.parse::<u64>().map_err(|_| {
                                AgentError::StepFailed(format!(
                                    "Invalid index parameter: must be numeric, got '{}'",
                                    index
                                ))
                            })?;
                            format!("[data-mcp-index=\"{}\"]", index_num)
                        } else {
                            return Err(AgentError::StepFailed(
                                "Missing 'selector' or 'index' parameter".into(),
                            ));
                        };
                        let text = action.parameters.get("text").ok_or_else(|| {
                            AgentError::StepFailed("Missing 'text' parameter".into())
                        })?;
                        (
                            "browser_type_text",
                            serde_json::json!({
                                "selector": selector,
                                "text": text,
                                "clear": true
                            }),
                        )
                    }
                    "scroll" => {
                        let direction = action
                            .parameters
                            .get("direction")
                            .map(|s| s.as_str())
                            .unwrap_or("down");

                        // Parse scroll amount with default fallback
                        let amount = action
                            .parameters
                            .get("amount")
                            .and_then(|a| a.parse::<i32>().ok())
                            .unwrap_or(500);

                        // Validate and clamp to reasonable range (1-10,000 pixels)
                        // Rationale: Typical viewport is ~1000-2000px tall, 10k = ~5 screen heights
                        let original_amount = amount;
                        let amount = amount.clamp(1, 10_000);

                        // Warn if value was clamped (helps debugging LLM behavior)
                        if original_amount != amount {
                            warn!(
                                "Scroll amount {} out of range [1, 10000], clamped to {}",
                                original_amount, amount
                            );
                        }

                        let (x, y) = match direction {
                            "up" => (0, -amount),
                            "down" => (0, amount),
                            "left" => (-amount, 0),
                            "right" => (amount, 0),
                            _ => (0, amount),
                        };

                        (
                            "browser_scroll",
                            serde_json::json!({
                                "x": x,
                                "y": y
                            }),
                        )
                    }
                    "extract_page_content" => ("browser_extract_text", serde_json::json!({})),
                    "done" => {
                        // Special case: mark completion without MCP call
                        // Agent protocol uses "done" to signal task completion
                        results.push(ActionResult {
                            action: "done".into(),
                            success: true,
                            extracted_content: action
                                .parameters
                                .get("result")
                                .map(|r| r.to_string())
                                .or_else(|| Some("Task completed".into())),
                            error: None,
                        });
                        continue;
                    }
                    _ => {
                        let error_msg = format!("Unknown action: {}", action.action);
                        warn!("Agent attempted unknown action: {}", action.action);
                        errors.push(error_msg.clone());
                        results.push(ActionResult {
                            action: action.action.clone(),
                            success: false,
                            extracted_content: None,
                            error: Some(error_msg),
                        });
                        continue;
                    }
                };

            // Call existing tool via MCP client (HOT PATH!)
            debug!(
                "Agent calling MCP tool: {} with args: {:?}",
                tool_name, tool_args
            );
            match self.mcp_client.call_tool(tool_name, tool_args).await {
                Ok(result) => {
                    info!(
                        "Tool {} succeeded for action '{}': {:?}",
                        tool_name, action.action, result
                    );

                    // Extract meaningful content from tool response
                    // Tools return text content in CallToolResult.content[0].text
                    let content = result
                        .content
                        .first()
                        .and_then(|c| c.as_text())
                        .map(|t| t.text.clone())
                        .unwrap_or_else(|| format!("Tool {} completed", tool_name));

                    results.push(ActionResult {
                        action: action.action,
                        success: true,
                        extracted_content: Some(content),
                        error: None,
                    });
                }
                Err(e) => {
                    let error_msg = format!(
                        "Tool '{}' failed for action '{}': {}",
                        tool_name, action.action, e
                    );
                    warn!("{}", error_msg);
                    errors.push(error_msg.clone());
                    results.push(ActionResult {
                        action: action.action,
                        success: false,
                        extracted_content: None,
                        error: Some(error_msg),
                    });
                }
            }
        }

        Ok((results, errors))
    }
}
/// Struct to hold browser state, screenshot path, and visual description
#[derive(Debug, Clone)]
struct BrowserStateWithScreenshot {
    state: String,
    screenshot_path: Option<String>,
    visual_description: Option<String>,
}
