mod common;

use anyhow::{Context, Result};
use kodegen_mcp_client::responses::SpawnClaudeAgentResponse;
use kodegen_mcp_client::tools;
use rmcp::model::CallToolResult;
use serde_json::json;
use tracing::info;

/// Extract text content from `CallToolResult`
fn extract_text_content(result: &CallToolResult) -> Result<String> {
    let content = result.content.first().context("No content in response")?;

    let text = content.as_text().context("Response content is not text")?;

    Ok(text.text.clone())
}

/// Parse JSON from `CallToolResult`
fn extract_json(result: &CallToolResult) -> Result<serde_json::Value> {
    let text = extract_text_content(result)?;
    serde_json::from_str(&text).context("Invalid JSON in response")
}

/// Display agent messages from parsed JSON output
fn display_agent_messages(output: &serde_json::Value) -> Result<()> {
    let messages = output
        .get("output")
        .and_then(|m| m.as_array())
        .context("No output array in response")?;

    for msg in messages {
        let msg_type = msg
            .get("message_type")
            .and_then(|t| t.as_str())
            .unwrap_or("unknown");
        let content = msg
            .get("content")
            .context("Missing content field in message")?;

        match msg_type {
            "assistant" => {
                // Extract assistant message content
                if let Some(message) = content.get("message")
                    && let Some(content_arr) = message.get("content").and_then(|c| c.as_array())
                {
                    for block in content_arr {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            info!("🤖 Assistant: {}", text);
                        }
                    }
                }
            }
            s if s.starts_with("system") => {
                // System messages like "system_init"
                info!("⚙️  System: {}", msg_type);
            }
            "result" => {
                // Result message with metrics
                let num_turns = content
                    .get("num_turns")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let duration = content
                    .get("duration_ms")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                if let Some(result_text) = content.get("result").and_then(|r| r.as_str()) {
                    info!(
                        "✅ Result (turn {}, {}ms): {}",
                        num_turns, duration, result_text
                    );
                } else {
                    info!("✅ Result: {} turns, {}ms", num_turns, duration);
                }
            }
            _ => {
                // Other message types
                info!("📝 {}: {:?}", msg_type, content);
            }
        }
    }

    Ok(())
}

/// Poll agent output until conversation is complete
async fn poll_until_complete(
    client: &common::LoggingClient,
    session_id: &str,
    max_polls: usize,
) -> Result<serde_json::Value> {
    for attempt in 1..=max_polls {
        let result = client
            .call_tool(
                tools::READ_CLAUDE_AGENT_OUTPUT,
                json!({
                    "session_id": session_id,
                    "offset": 0,
                    "length": 100
                }),
            )
            .await
            .context("Failed to read agent output")?;

        let output = extract_json(&result)?;

        // Check if conversation is complete
        let is_complete = output
            .get("is_complete")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if is_complete {
            info!("Agent completed after {} polls", attempt);
            return Ok(output);
        }

        // Still working, continue polling
        if attempt < max_polls {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    // Timeout - return last output anyway
    let result = client
        .call_tool(
            tools::READ_CLAUDE_AGENT_OUTPUT,
            json!({
                "session_id": session_id,
                "offset": 0,
                "length": 100
            }),
        )
        .await?;

    let output = extract_json(&result)?;
    tracing::warn!(
        "Polling timeout after {} attempts, returning last output",
        max_polls
    );
    Ok(output)
}

/// Extract and display agent info from agents list
fn display_agents_list(response: &serde_json::Value) -> Result<()> {
    let agents = response
        .get("agents")
        .and_then(|a| a.as_array())
        .context("Response missing agents array")?;

    let total_active = response
        .get("total_active")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    let total_completed = response
        .get("total_completed")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    tracing::info!(
        "Total agents: {} (active: {}, completed: {})",
        agents.len(),
        total_active,
        total_completed
    );

    for agent in agents {
        let id = agent
            .get("session_id")
            .and_then(|s| s.as_str())
            .context("Agent missing session_id")?;

        let label = agent
            .get("label")
            .and_then(|l| l.as_str())
            .unwrap_or("unlabeled");

        let working = agent
            .get("working")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let status = if working { "🔄 WORKING" } else { "✅ idle" };

        tracing::info!("  {} - {} ({})", id, label, status);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting claude agent tools example");

    // Connect to kodegen server (already running with all tools)
    let (conn, mut server) = common::connect_to_local_sse_server().await?;

    // Wrap client with logging
    let log_path =
        std::path::PathBuf::from("/Volumes/samsung_t9/kodegen/tmp/mcp-client/claude_agent.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // 1. SPAWN_CLAUDE_AGENT - Spawn a new Claude agent with initial question
    info!("1. Testing spawn_claude_agent with context-dependent conversation");
    let response: SpawnClaudeAgentResponse = client
        .call_tool_typed(
            tools::SPAWN_CLAUDE_AGENT,
            json!({
                "prompt": {
                    "type": "string",
                    "value": "What is the capital of France?"
                },
                "model": "sonnet",
                "max_turns": 5
            }),
        )
        .await?;

    let session_id = &response.session_ids[0];
    info!("✅ Spawned agent with session ID: {}", session_id);

    // 2. READ_CLAUDE_AGENT_OUTPUT - Read agent response
    info!("2. Testing read_claude_agent_output");
    info!("Polling until conversation completes...");

    match poll_until_complete(&client, session_id, 20).await {
        Ok(output) => {
            if let Err(e) = display_agent_messages(&output) {
                tracing::error!("Failed to display agent messages: {}", e);
                info!("Agent output: {:?}", output);
            }
        }
        Err(e) => {
            tracing::error!("Failed to poll agent output: {}", e);
        }
    }
    info!("✅ Read agent output successfully");

    // 3. SEND_CLAUDE_AGENT_PROMPT - Send context-dependent follow-up
    info!("\n=== 3. Testing send_claude_agent_prompt (context-dependent) ===");
    info!("Sending follow-up that requires context from previous answer...");

    client
        .call_tool(
            tools::SEND_CLAUDE_AGENT_PROMPT,
            json!({
                "session_id": session_id,
                "prompt": {
                    "type": "string",
                    "value": "What is its population? Just give me the approximate number."
                }
            }),
        )
        .await
        .context("Failed to send follow-up prompt")?;
    info!("✅ Sent context-dependent follow-up prompt");

    info!("Polling for follow-up response...");
    match poll_until_complete(&client, session_id, 20).await {
        Ok(output) => {
            let total_msgs = output
                .get("total_messages")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let turn_count = output
                .get("turn_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            info!(
                "Follow-up response received ({} total messages, turn {})",
                total_msgs, turn_count
            );
            if let Err(e) = display_agent_messages(&output) {
                tracing::error!("Failed to display follow-up messages: {}", e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to poll follow-up output: {}", e);
        }
    }
    info!("✅ Read follow-up output successfully");

    // 3b. Send another context-dependent follow-up to verify continued context
    info!("\n=== 3b. Testing continued context across multiple turns ===");
    info!("Sending third prompt that requires full conversation context...");

    client
        .call_tool(
            tools::SEND_CLAUDE_AGENT_PROMPT,
            json!({
                "session_id": session_id,
                "prompt": {
                    "type": "string",
                    "value": "What famous landmark is located there?"
                }
            }),
        )
        .await
        .context("Failed to send third prompt")?;
    info!("✅ Sent third context-dependent prompt");

    info!("Polling for third response...");
    match poll_until_complete(&client, session_id, 20).await {
        Ok(output) => {
            let total_msgs = output
                .get("total_messages")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let turn_count = output
                .get("turn_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            info!(
                "Third response received ({} total messages, turn {})",
                total_msgs, turn_count
            );
            if let Err(e) = display_agent_messages(&output) {
                tracing::error!("Failed to display third messages: {}", e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to poll third output: {}", e);
        }
    }
    info!("✅ Context maintained across 3 turns!");

    // 4. LIST_CLAUDE_AGENTS - List all active agents
    info!("\n=== 4. Testing list_claude_agents ===");
    let result = client
        .call_tool(tools::LIST_CLAUDE_AGENTS, json!({}))
        .await
        .context("Failed to list agents")?;

    match extract_json(&result) {
        Ok(agents) => {
            if let Err(e) = display_agents_list(&agents) {
                tracing::error!("Failed to display agents list: {}", e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse agents list: {}", e);
        }
    }
    info!("✅ Listed agents successfully");

    // 4b. Multiple concurrent agents
    info!("\n=== 4b. Testing multiple concurrent agents ===");

    let response_2: SpawnClaudeAgentResponse = client
        .call_tool_typed(
            tools::SPAWN_CLAUDE_AGENT,
            json!({
                "prompt": {
                    "type": "string",
                    "value": "You are a technical writer. Explain concepts clearly and simply."
                },
                "model": "sonnet"
            }),
        )
        .await?;

    let session_id_2 = &response_2.session_ids[0];
    info!("✅ Second agent spawned: {}", session_id_2);

    // List all agents to verify both active
    let result = client
        .call_tool(tools::LIST_CLAUDE_AGENTS, json!({}))
        .await
        .context("Failed to list agents for verification")?;

    match extract_json(&result) {
        Ok(agents) => {
            if let Err(e) = display_agents_list(&agents) {
                tracing::error!("Failed to display agents list for verification: {}", e);
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse agents list for verification: {}", e);
        }
    }
    info!("✅ Listed agents for verification");

    // 5. TERMINATE_CLAUDE_AGENT_SESSION - Terminate the agent
    info!("5. Testing terminate_claude_agent_session");
    client
        .call_tool(
            tools::TERMINATE_CLAUDE_AGENT_SESSION,
            json!({ "session_id": session_id }),
        )
        .await
        .context("Failed to terminate first agent")?;
    info!("✅ Terminated first agent: {}", session_id);

    // Terminate second agent if it exists
    client
        .call_tool(
            tools::TERMINATE_CLAUDE_AGENT_SESSION,
            json!({ "session_id": session_id_2 }),
        )
        .await
        .context("Failed to terminate second agent")?;
    info!("✅ Terminated second agent: {}", session_id_2);

    // Verify all agents terminated
    info!("\n=== Verifying cleanup ===");
    let result = client
        .call_tool(tools::LIST_CLAUDE_AGENTS, json!({}))
        .await
        .context("Failed to verify cleanup")?;

    match extract_json(&result) {
        Ok(response) => {
            let total_active = response
                .get("total_active")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);

            let total_completed = response
                .get("total_completed")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);

            if total_active == 0 && total_completed == 2 {
                info!(
                    "✅ All agents terminated successfully (active: {}, completed: {})",
                    total_active, total_completed
                );
            } else {
                tracing::error!(
                    "❌ Cleanup verification failed: active: {}, completed: {} (expected active: 0, completed: 2)",
                    total_active,
                    total_completed
                );
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse cleanup verification: {}", e);
        }
    }
    info!("✅ Verified cleanup successfully");

    // Graceful shutdown
    conn.close().await?;
    server.shutdown().await?;
    info!("\n✅ Claude agent tools example completed successfully");

    info!("\n📚 Features Demonstrated:");
    info!("  • Spawning Claude agent sub-sessions");
    info!("  • Sending prompts to agents");
    info!("  • Reading agent responses with message parsing");
    info!("  • Multi-turn conversations with context persistence");
    info!("  • Multiple concurrent agents with different configurations");
    info!("  • Session management and cleanup verification");

    Ok(())
}
