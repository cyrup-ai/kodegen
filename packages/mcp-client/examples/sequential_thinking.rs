mod common;

use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting sequential thinking tool example");

    // Connect to kodegen server with sequential_thinking category
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::SequentialThinking])
    ).await?;

    info!("Connected to server: {:?}", client.server_info());

    // 1. SEQUENTIAL_THINKING - Demonstrate step-by-step reasoning
    info!("1. Testing sequential_thinking");
    
    // First thought
    match client.call_tool(
        tools::SEQUENTIAL_THINKING,
        json!({
            "thought": "I need to solve the problem: What is 15 * 24?",
            "thought_number": 1,
            "total_thoughts": 3,
            "next_thought_needed": true
        })
    ).await {
        Ok(result) => info!("Thought 1: {:?}", result),
        Err(e) => error!("Failed on thought 1: {}", e),
    }

    // Second thought
    match client.call_tool(
        tools::SEQUENTIAL_THINKING,
        json!({
            "thought": "Let me break this down: 15 * 24 = 15 * (20 + 4) = (15*20) + (15*4)",
            "thought_number": 2,
            "total_thoughts": 3,
            "next_thought_needed": true
        })
    ).await {
        Ok(result) => info!("Thought 2: {:?}", result),
        Err(e) => error!("Failed on thought 2: {}", e),
    }

    // Third thought (final)
    match client.call_tool(
        tools::SEQUENTIAL_THINKING,
        json!({
            "thought": "Computing: 15*20 = 300, and 15*4 = 60. Therefore 300 + 60 = 360. Answer: 360",
            "thought_number": 3,
            "total_thoughts": 3,
            "next_thought_needed": false
        })
    ).await {
        Ok(result) => info!("Thought 3 (final): {:?}", result),
        Err(e) => error!("Failed on thought 3: {}", e),
    }

    // Graceful shutdown
    client.close().await?;
    info!("Sequential thinking tool example completed successfully");

    Ok(())
}
