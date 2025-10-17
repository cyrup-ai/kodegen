mod common;

use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{info, error};
use tokio::time::{sleep, Duration};

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
    let spawn_result = client.call_tool(
        tools::SPAWN_CLAUDE_AGENT,
        json!({
            "prompt": "You are a helpful assistant. Answer questions concisely.",
            "model": "claude-3-5-sonnet-20241022"
        })
    ).await;

    let session_id = if let Ok(result) = &spawn_result {
        // Parse session_id from CallToolResult content
        let sid = result.content.first()
            .and_then(|content| content.as_text())
            .and_then(|text_content| {
                serde_json::from_str::<serde_json::Value>(&text_content.text)
                    .ok()
                    .and_then(|v| v["session_id"].as_str().map(String::from))
            });
        
        if let Some(ref s) = sid {
            info!("✅ Spawned agent with session ID: {}", s);
        } else {
            info!("Spawned agent: {:?}", result);
        }
        sid
    } else {
        error!("Failed to spawn agent: {:?}", spawn_result);
        None
    };

    if let Some(sid) = session_id {
        // 2. SEND_CLAUDE_AGENT_PROMPT - Send a prompt to the agent
        info!("2. Testing send_claude_agent_prompt");
        match client.call_tool(
            tools::SEND_CLAUDE_AGENT_PROMPT,
            json!({
                "session_id": sid,
                "prompt": "What is 2 + 2?"
            })
        ).await {
            Ok(result) => info!("Sent prompt: {:?}", result),
            Err(e) => error!("Failed to send prompt: {}", e),
        }

        // 3. READ_CLAUDE_AGENT_OUTPUT - Read agent response
        info!("3. Testing read_claude_agent_output");
        match client.call_tool(
            tools::READ_CLAUDE_AGENT_OUTPUT,
            json!({
                "session_id": sid,
                "timeout_ms": 5000
            })
        ).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text() {
                    if let Ok(output) = serde_json::from_str::<serde_json::Value>(&text.text) {
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
                    } else {
                        info!("Agent output: {:?}", result);
                    }
                }
            },
            Err(e) => error!("Failed to read output: {}", e),
        }
        
        // 3b. Multi-turn conversation
        info!("\n=== Testing multi-turn conversation ===");
        
        sleep(Duration::from_secs(2)).await;
        
        match client.call_tool(
            tools::SEND_CLAUDE_AGENT_PROMPT,
            json!({
                "session_id": sid,
                "prompt": "Now explain it in simpler terms for a beginner."
            })
        ).await {
            Ok(_) => info!("Sent follow-up prompt"),
            Err(e) => error!("Failed to send follow-up prompt: {}", e),
        }
        
        sleep(Duration::from_secs(2)).await;
        
        match client.call_tool(
            tools::READ_CLAUDE_AGENT_OUTPUT,
            json!({
                "session_id": sid,
                "timeout_ms": 5000
            })
        ).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text()
                    && let Ok(output) = serde_json::from_str::<serde_json::Value>(&text.text)
                    && let Some(messages) = output.get("messages").and_then(|m| m.as_array()) {
                    info!("Follow-up response received ({} messages)", messages.len());
                }
            },
            Err(e) => error!("Failed to read follow-up output: {}", e),
        }

        // 4. LIST_CLAUDE_AGENTS - List all active agents
        info!("4. Testing list_claude_agents");
        match client.call_tool(tools::LIST_CLAUDE_AGENTS, json!({})).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text() {
                    if let Ok(agents) = serde_json::from_str::<serde_json::Value>(&text.text) {
                        if let Some(arr) = agents.as_array() {
                            info!("Total active agents: {}", arr.len());
                            for agent in arr {
                                let id = agent.get("sessionId").and_then(|s| s.as_str()).unwrap_or("unknown");
                                let model = agent.get("model").and_then(|m| m.as_str()).unwrap_or("unknown");
                                info!("  Agent {}: {}", id, model);
                            }
                        }
                    } else {
                        info!("Active agents: {:?}", result);
                    }
                }
            },
            Err(e) => error!("Failed to list agents: {}", e),
        }
        
        // 4b. Multiple concurrent agents
        info!("\n=== Testing multiple concurrent agents ===");
        
        let spawn_result_2 = client.call_tool(
            tools::SPAWN_CLAUDE_AGENT,
            json!({
                "prompt": "You are a technical writer. Explain concepts clearly and simply.",
                "model": "claude-3-5-sonnet-20241022",
                "maxTokens": 512
            })
        ).await;
        
        let session_id_2 = if let Ok(result) = &spawn_result_2 {
            result.content.first()
                .and_then(|content| content.as_text())
                .and_then(|text_content| {
                    serde_json::from_str::<serde_json::Value>(&text_content.text)
                        .ok()
                        .and_then(|v| v["session_id"].as_str().map(String::from))
                })
        } else {
            error!("Failed to spawn second agent: {:?}", spawn_result_2);
            None
        };
        
        if let Some(sid2) = session_id_2.as_ref() {
            info!("✅ Second agent spawned: {}", sid2);
            
            // List all agents to verify both active
            match client.call_tool(tools::LIST_CLAUDE_AGENTS, json!({})).await {
                Ok(result) => {
                    if let Some(content) = result.content.first()
                        && let Some(text) = content.as_text()
                        && let Ok(agents) = serde_json::from_str::<serde_json::Value>(&text.text)
                        && let Some(arr) = agents.as_array() {
                        info!("Total active agents: {}", arr.len());
                        for agent in arr {
                            let id = agent.get("sessionId").and_then(|s| s.as_str()).unwrap_or("unknown");
                            let model = agent.get("model").and_then(|m| m.as_str()).unwrap_or("unknown");
                            info!("  Agent {}: {}", id, model);
                        }
                    }
                },
                Err(e) => error!("Failed to list agents: {}", e),
            }
        }

        // 5. TERMINATE_CLAUDE_AGENT_SESSION - Terminate the agent
        info!("5. Testing terminate_claude_agent_session");
        match client.call_tool(
            tools::TERMINATE_CLAUDE_AGENT_SESSION,
            json!({ "session_id": sid })
        ).await {
            Ok(_) => info!("✅ Terminated first agent: {}", sid),
            Err(e) => error!("Failed to terminate agent: {}", e),
        }
        
        // Terminate second agent if it exists
        if let Some(sid2) = session_id_2 {
            match client.call_tool(
                tools::TERMINATE_CLAUDE_AGENT_SESSION,
                json!({ "session_id": sid2 })
            ).await {
                Ok(_) => info!("✅ Terminated second agent: {}", sid2),
                Err(e) => error!("Failed to terminate second agent: {}", e),
            }
        }
        
        // Verify all agents terminated
        info!("\n=== Verifying cleanup ===");
        match client.call_tool(tools::LIST_CLAUDE_AGENTS, json!({})).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text()
                    && let Ok(agents) = serde_json::from_str::<serde_json::Value>(&text.text)
                    && let Some(arr) = agents.as_array() {
                    info!("Active agents after termination: {}", arr.len());
                }
            },
            Err(e) => error!("Failed to verify cleanup: {}", e),
        }
    }

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
