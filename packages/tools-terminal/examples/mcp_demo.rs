mod common;

use anyhow::Context;
use kodegen_mcp_client::responses::StartTerminalCommandResponse;
use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting terminal tools example");

    // Connect to kodegen server with terminal category
    let (conn, mut server) =
        common::connect_to_local_sse_server().await?;
            .await?;

    // Wrap client with logging
    let workspace_root = common::find_workspace_root()
        .context("Failed to find workspace root")?;
    let log_path = workspace_root.join("tmp/mcp-client/terminal.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // Run example with guaranteed cleanup
    let result = run_terminal_example(&client).await;

    // Always close connection, regardless of example result
    conn.close().await?;
    server.shutdown().await?;

    // Propagate any error from the example AFTER cleanup
    result
}

async fn run_terminal_example(client: &common::LoggingClient) -> anyhow::Result<()> {
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
                "initial_delay_ms": 5000
            })
        ).await?;
        
        let pid = response.pid;
        pids.push(pid);
        info!("Started command with PID: {}", pid);
        
        // 2. READ_TERMINAL_OUTPUT - Read command output
        info!("2. Testing read_terminal_output");
        match client.call_tool(
            tools::READ_TERMINAL_OUTPUT,
            json!({ "pid": pid })
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
                "command": "cat"  // cat waits for input - uses default 100ms initial_delay_ms
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

        // 6. PTY FUNCTIONALITY TEST - Run top and poll multiple times to see dynamic updates
        info!("\n6. Testing pseudo-terminal with top (dynamic output)");
        let top_response: StartTerminalCommandResponse = client.call_tool_typed(
            tools::START_TERMINAL_COMMAND,
            json!({
                "command": "top -l 3 -s 1"  // 3 samples, 1 second apart - uses default 100ms initial_delay_ms
            })
        ).await?;
        
        let top_pid = top_response.pid;
        pids.push(top_pid);
        info!("Started top with PID: {}", top_pid);

        // Poll 1 - Should show first sample
        info!("   Poll 1 (after 1 second):");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        match client.call_tool(
            tools::READ_TERMINAL_OUTPUT,
            json!({ "pid": top_pid })
        ).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text() {
                        let lines: Vec<&str> = text.text.lines().take(10).collect();
                        info!("   First 10 lines:\n{}", lines.join("\n"));
                    }
            },
            Err(e) => error!("   Failed to read top output (poll 1): {}", e),
        }

        // Poll 2 - Should show second sample with updated stats
        info!("   Poll 2 (after 2 seconds):");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        match client.call_tool(
            tools::READ_TERMINAL_OUTPUT,
            json!({ "pid": top_pid })
        ).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text() {
                        let lines: Vec<&str> = text.text.lines().take(10).collect();
                        info!("   First 10 lines:\n{}", lines.join("\n"));
                    }
            },
            Err(e) => error!("   Failed to read top output (poll 2): {}", e),
        }

        // Poll 3 - Should show third sample
        info!("   Poll 3 (after 3 seconds):");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        match client.call_tool(
            tools::READ_TERMINAL_OUTPUT,
            json!({ "pid": top_pid })
        ).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text() {
                        let lines: Vec<&str> = text.text.lines().take(10).collect();
                        info!("   First 10 lines:\n{}", lines.join("\n"));
                    }
            },
            Err(e) => error!("   Failed to read top output (poll 3): {}", e),
        }

        // Kill top process
        info!("   Stopping top process:");
        match client.call_tool(
            tools::STOP_TERMINAL_COMMAND,
            json!({ "pid": top_pid })
        ).await {
            Ok(result) => info!("   Stopped top: {:?}", result),
            Err(e) => error!("   Failed to stop top: {}", e),
        }

        // 7. MULTI-STEP SESSION TEST - Interactive bash session with command history
        info!("\n7. Testing multi-step interactive session (bash with history)");
        let bash_response: StartTerminalCommandResponse = client.call_tool_typed(
            tools::START_TERMINAL_COMMAND,
            json!({
                "command": "bash"  // Uses default 100ms initial_delay_ms
            })
        ).await?;
        
        let bash_pid = bash_response.pid;
        pids.push(bash_pid);
        info!("Started bash session with PID: {}", bash_pid);

        // Wait for bash to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Command 1: echo something
        info!("   Sending command 1: echo 'First command'");
        match client.call_tool(
            tools::SEND_TERMINAL_INPUT,
            json!({
                "pid": bash_pid,
                "input": "echo 'First command'\n"
            })
        ).await {
            Ok(result) => info!("   Sent command 1: {:?}", result),
            Err(e) => error!("   Failed to send command 1: {}", e),
        }

        // Wait for command to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Read output from command 1
        info!("   Reading output from command 1:");
        match client.call_tool(
            tools::READ_TERMINAL_OUTPUT,
            json!({ "pid": bash_pid })
        ).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text() {
                        info!("   Output:\n{}", text.text);
                    }
            },
            Err(e) => error!("   Failed to read output: {}", e),
        }

        // Command 2: run history to show session is maintained
        info!("   Sending command 2: history");
        match client.call_tool(
            tools::SEND_TERMINAL_INPUT,
            json!({
                "pid": bash_pid,
                "input": "history\n"
            })
        ).await {
            Ok(result) => info!("   Sent command 2: {:?}", result),
            Err(e) => error!("   Failed to send command 2: {}", e),
        }

        // Wait for history command to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Read history output - should show previous echo command
        info!("   Reading history output (should show 'echo First command'):");
        match client.call_tool(
            tools::READ_TERMINAL_OUTPUT,
            json!({ "pid": bash_pid })
        ).await {
            Ok(result) => {
                if let Some(content) = result.content.first()
                    && let Some(text) = content.as_text() {
                        info!("   History output:\n{}", text.text);
                        if text.text.contains("echo 'First command'") || text.text.contains("echo First command") {
                            info!("   ✅ Session state maintained - history shows previous command!");
                        } else {
                            error!("   ⚠️  History does not show previous command - session state may not be maintained");
                        }
                    }
            },
            Err(e) => error!("   Failed to read history: {}", e),
        }

        // Stop bash session
        info!("   Stopping bash session:");
        match client.call_tool(
            tools::STOP_TERMINAL_COMMAND,
            json!({ "pid": bash_pid })
        ).await {
            Ok(result) => info!("   Stopped bash: {:?}", result),
            Err(e) => error!("   Failed to stop bash: {}", e),
        }

        info!("\nTerminal tools example tests completed");
        Ok::<(), anyhow::Error>(())
    }.await;

    // Always cleanup terminal processes, regardless of test result
    cleanup_terminal_processes(client, &pids).await;

    // Propagate test result AFTER cleanup
    test_result
}

async fn cleanup_terminal_processes(client: &common::LoggingClient, pids: &[i64]) {
    info!("\nCleaning up processes...");

    for process_pid in pids {
        match client
            .call_tool(tools::STOP_TERMINAL_COMMAND, json!({ "pid": process_pid }))
            .await
        {
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
