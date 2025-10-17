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

    info!("Starting terminal tools example");

    // Connect to kodegen server with terminal category
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Terminal])
    ).await?;

    info!("Connected to server: {:?}", client.server_info());

    // 1. START_TERMINAL_COMMAND - Start an echo command
    info!("1. Testing start_terminal_command");
    let result = client.call_tool(
        tools::START_TERMINAL_COMMAND,
        json!({
            "command": "echo 'Hello from terminal'",
            "timeout_ms": 5000
        })
    ).await;
    
    let pid = if let Ok(result) = &result {
        info!("Started command: {:?}", result);
        // Parse PID from CallToolResult content
        result.content.first()
            .and_then(|content| content.as_text())
            .and_then(|text_content| {
                serde_json::from_str::<serde_json::Value>(&text_content.text)
                    .ok()
                    .and_then(|v| v["pid"].as_i64())
            })
    } else {
        error!("Failed to start command: {:?}", result);
        None
    };

    if let Some(pid) = pid {
        // 2. READ_TERMINAL_OUTPUT - Read command output
        info!("2. Testing read_terminal_output");
        match client.call_tool(
            tools::READ_TERMINAL_OUTPUT,
            json!({ "pid": pid, "timeout_ms": 1000 })
        ).await {
            Ok(result) => info!("Read output: {:?}", result),
            Err(e) => error!("Failed to read output: {}", e),
        }

        // 3. SEND_TERMINAL_INPUT - Send input to an interactive command
        info!("3. Testing send_terminal_input");
        // Start an interactive command first
        let interactive_result = client.call_tool(
            tools::START_TERMINAL_COMMAND,
            json!({
                "command": "cat",  // cat waits for input
                "timeout_ms": 1000
            })
        ).await;
        
        if let Ok(interactive_result) = interactive_result {
            // Parse PID from the interactive command result
            let interactive_pid = interactive_result.content.first()
                .and_then(|content| content.as_text())
                .and_then(|text_content| {
                    serde_json::from_str::<serde_json::Value>(&text_content.text)
                        .ok()
                        .and_then(|v| v["pid"].as_i64())
                });
            
            if let Some(interactive_pid) = interactive_pid {
                match client.call_tool(
                tools::SEND_TERMINAL_INPUT,
                json!({
                    "pid": interactive_pid,
                    "input": "test input\n"
                })
            ).await {
                Ok(result) => info!("Sent input: {:?}", result),
                Err(e) => error!("Failed to send input: {}", e),
            }

            // 4. STOP_TERMINAL_COMMAND - Stop the interactive command
            info!("4. Testing stop_terminal_command");
            match client.call_tool(
                tools::STOP_TERMINAL_COMMAND,
                json!({ "pid": interactive_pid })
            ).await {
                Ok(result) => info!("Stopped command: {:?}", result),
                Err(e) => error!("Failed to stop command: {}", e),
            }
            }
        }

        // 5. LIST_TERMINAL_COMMANDS - List all active sessions
        info!("5. Testing list_terminal_commands");
        match client.call_tool(tools::LIST_TERMINAL_COMMANDS, json!({})).await {
            Ok(result) => info!("Active sessions: {:?}", result),
            Err(e) => error!("Failed to list sessions: {}", e),
        }
    }

    // Graceful shutdown
    client.close().await?;
    info!("Terminal tools example completed successfully");

    Ok(())
}
