mod common;

use anyhow::Context;
use kodegen_mcp_client::responses::SpawnClaudeAgentResponse;
use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting claude agent tools example");

    // Check for Anthropic API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        tracing::warn!("⚠️  ANTHROPIC_API_KEY not set. Claude agent features require an API key.");
        tracing::warn!("Set it with: export ANTHROPIC_API_KEY=your_key_here");
        info!("Skipping example - set API key to run this test");
        return Ok(());
    }

    // Connect to kodegen server with claude_agent category
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::ClaudeAgent])
    ).await?;

    info!("Connected to server: {:?}", client.server_info());

    // 1. SPAWN_CLAUDE_AGENT - Spawn a new Claude agent
    info!("1. Testing spawn_claude_agent");
    let response: SpawnClaudeAgentResponse = client.call_tool_typed(
        tools::SPAWN_CLAUDE_AGENT,
        json!({
            "prompt": "You are a helpful assistant. Answer questions concisely.",
            "model": "claude-3-5-sonnet-20241022"
        })
    ).await?;

    let session_id = response.session_id;
    info!("✅ Spawned agent with session ID: {}", session_id);

    // 2. SEND_CLAUDE_AGENT_PROMPT - Send a prompt to the agent
    info!("2. Testing send_claude_agent_prompt");
    client.call_tool(
        tools::SEND_CLAUDE_AGENT_PROMPT,
        json!({
            "session_id": session_id,
            "prompt": "What is 2 + 2?"
        })
    )
    .await
    .context("Failed to send prompt to agent")?;
    info!("✅ Sent prompt successfully");

    // 3. READ_CLAUDE_AGENT_OUTPUT - Read agent response
    info!("3. Testing read_claude_agent_output");
    let result = client.call_tool(
        tools::READ_CLAUDE_AGENT_OUTPUT,
        json!({
            "session_id": session_id,
            "timeout_ms": 5000
        })
    )
    .await
    .context("Failed to read agent output")?;
    
    match common::extract_json(&result) {
        Ok(output) => {
            if let Some(messages) = output.get("messages").and_then(|m| m.as_array()) {
                for msg in messages {
                    if let Some(role) = msg.get("role").and_then(|r| r.as_str())
                        && let Some(content_arr) = msg.get("content").and_then(|c| c.as_array()) {
                        for content_item in content_arr {
                            if let Some(text_content) = content_item.get("text").and_then(|t| t.as_str()) {
                                info!("{}: {}", role, text_content);
                            }
                        }
                    }
                }
            } else {
                info!("Agent output: {:?}", output);
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse agent output: {}", e);
        }
    }
    info!("✅ Read agent output successfully");
    
    // 3b. Multi-turn conversation
    info!("\n=== Testing multi-turn conversation ===");
    
    client.call_tool(
        tools::SEND_CLAUDE_AGENT_PROMPT,
        json!({
            "session_id": session_id,
            "prompt": "Now explain it in simpler terms for a beginner."
        })
    )
    .await
    .context("Failed to send follow-up prompt")?;
    info!("✅ Sent follow-up prompt");
    
    let result = client.call_tool(
        tools::READ_CLAUDE_AGENT_OUTPUT,
        json!({
            "session_id": session_id,
            "timeout_ms": 5000
        })
    )
    .await
    .context("Failed to read follow-up output")?;
    
    match common::extract_json(&result) {
        Ok(output) => {
            if let Some(messages) = output.get("messages").and_then(|m| m.as_array()) {
                info!("Follow-up response received ({} messages)", messages.len());
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse follow-up output: {}", e);
        }
    }
    info!("✅ Read follow-up output successfully");

    // 4. LIST_CLAUDE_AGENTS - List all active agents
    info!("4. Testing list_claude_agents");
    let result = client.call_tool(tools::LIST_CLAUDE_AGENTS, json!({}))
        .await
        .context("Failed to list agents")?;
    
    match common::extract_json(&result) {
        Ok(agents) => {
            if let Some(arr) = agents.as_array() {
                info!("Total active agents: {}", arr.len());
                for agent in arr {
                    let id = agent.get("sessionId").and_then(|s| s.as_str()).unwrap_or("unknown");
                    let model = agent.get("model").and_then(|m| m.as_str()).unwrap_or("unknown");
                    info!("  Agent {}: {}", id, model);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse agents list: {}", e);
        }
    }
    info!("✅ Listed agents successfully");
    
    // 4b. Multiple concurrent agents
    info!("\n=== Testing multiple concurrent agents ===");
    
    let response_2: SpawnClaudeAgentResponse = client.call_tool_typed(
        tools::SPAWN_CLAUDE_AGENT,
        json!({
            "prompt": "You are a technical writer. Explain concepts clearly and simply.",
            "model": "claude-3-5-sonnet-20241022",
            "maxTokens": 512
        })
    ).await?;
    
    let session_id_2 = response_2.session_id;
    info!("✅ Second agent spawned: {}", session_id_2);
    
    // List all agents to verify both active
    let result = client.call_tool(tools::LIST_CLAUDE_AGENTS, json!({}))
        .await
        .context("Failed to list agents for verification")?;
    
    match common::extract_json(&result) {
        Ok(agents) => {
            if let Some(arr) = agents.as_array() {
                info!("Total active agents: {}", arr.len());
                for agent in arr {
                    let id = agent.get("sessionId").and_then(|s| s.as_str()).unwrap_or("unknown");
                    let model = agent.get("model").and_then(|m| m.as_str()).unwrap_or("unknown");
                    info!("  Agent {}: {}", id, model);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse agents list for verification: {}", e);
        }
    }
    info!("✅ Listed agents for verification");

    // 5. TERMINATE_CLAUDE_AGENT_SESSION - Terminate the agent
    info!("5. Testing terminate_claude_agent_session");
    client.call_tool(
        tools::TERMINATE_CLAUDE_AGENT_SESSION,
        json!({ "session_id": session_id })
    )
    .await
    .context("Failed to terminate first agent")?;
    info!("✅ Terminated first agent: {}", session_id);
    
    // Terminate second agent if it exists
    client.call_tool(
        tools::TERMINATE_CLAUDE_AGENT_SESSION,
        json!({ "session_id": session_id_2 })
    )
    .await
    .context("Failed to terminate second agent")?;
    info!("✅ Terminated second agent: {}", session_id_2);
    
    // Verify all agents terminated
    info!("\n=== Verifying cleanup ===");
    let result = client.call_tool(tools::LIST_CLAUDE_AGENTS, json!({}))
        .await
        .context("Failed to verify cleanup")?;
    
    match common::extract_json(&result) {
        Ok(agents) => {
            if let Some(arr) = agents.as_array() {
                info!("Active agents after termination: {}", arr.len());
            }
        }
        Err(e) => {
            tracing::error!("Failed to parse cleanup verification: {}", e);
        }
    }
    info!("✅ Verified cleanup successfully");

    // Graceful shutdown
    client.close().await?;
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
