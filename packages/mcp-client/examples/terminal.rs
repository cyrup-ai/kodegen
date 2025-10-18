mod common;

use kodegen_mcp_client::responses::StartTerminalCommandResponse;
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

    // Run example with guaranteed cleanup
    let result = run_terminal_example(&client).await;

    // Always close client, regardless of example result
    client.close().await?;

    // Propagate any error from the example AFTER cleanup
    result
}

async fn run_terminal_example(client: &kodegen_mcp_client::KodegenClient) -> anyhow::Result<()> {
    // Track PIDs for cleanup
    let mut pids = Vec::new();
    
    // Run all tests, capturing result
    let test_result = async {
        // 1. START_TERMINAL_COMMAND - Start an echo command
        info!("1. Testing start_terminal_command");
        let response: StartTerminalCommandResponse = client.call_tool_typed(
            tools::START_TERMINAL_COMMAND,
            json!({
                "command": "echo 'Hello from terminal'",
                "timeout_ms": 5000
            })
        ).await?;
        
        let pid = response.pid;
        pids.push(pid);
        info!("Started command with PID: {}", pid);
        
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
        let interactive_response: StartTerminalCommandResponse = client.call_tool_typed(
            tools::START_TERMINAL_COMMAND,
            json!({
                "command": "cat",  // cat waits for input
                "timeout_ms": 1000
            })
        ).await?;
        
        let interactive_pid = interactive_response.pid;
        pids.push(interactive_pid);
        info!("Started interactive command with PID: {}", interactive_pid);
        
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

        // 5. LIST_TERMINAL_COMMANDS - List all active sessions
        info!("5. Testing list_terminal_commands");
        match client.call_tool(tools::LIST_TERMINAL_COMMANDS, json!({})).await {
            Ok(result) => info!("Active sessions: {:?}", result),
            Err(e) => error!("Failed to list sessions: {}", e),
        }

        info!("Terminal tools example tests completed");
        Ok::<(), anyhow::Error>(())
    }.await;

    // Always cleanup terminal processes, regardless of test result
    cleanup_terminal_processes(client, &pids).await;

    // Propagate test result AFTER cleanup
    test_result
}

async fn cleanup_terminal_processes(client: &kodegen_mcp_client::KodegenClient, pids: &[i64]) {
    info!("\nCleaning up processes...");
    
    for process_pid in pids {
        match client.call_tool(
            tools::STOP_TERMINAL_COMMAND,
            json!({ "pid": process_pid })
        ).await {
            Ok(_) => info!("✅ Stopped process with PID: {}", process_pid),
            Err(e) => {
                // Process may have already exited, which is fine
                if e.to_string().contains("not found") || e.to_string().contains("No session") {
                    info!("Process {} already exited", process_pid);
                } else {
                    error!("⚠️  Failed to stop process {}: {}", process_pid, e);
                }
            }
        }
    }
}
