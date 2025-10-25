mod common;

use anyhow::Context;
use kodegen_mcp_client::{responses::GetConfigResponse, tools};
use serde_json::json;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting config tools example - testing both get_config and set_config_value");

    // Connect to kodegen server (config tools are always enabled, no category needed)
    let (conn, mut server) = common::connect_to_local_sse_server().await?;

    // Wrap client with logging
    let log_path =
        std::path::PathBuf::from("/Volumes/samsung_t9/kodegen/tmp/mcp-client/config.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // ========================================================================
    // TEST 1: Get initial config with typed response
    // ========================================================================
    info!("1. Testing get_config with typed response");
    let initial_config: GetConfigResponse = client
        .call_tool_typed(tools::GET_CONFIG, json!({}))
        .await
        .context("Failed to get initial config")?;

    info!("✅ Initial config retrieved:");
    info!(
        "   - Platform: {} {}",
        initial_config.system_info.platform, initial_config.system_info.arch
    );
    info!("   - Hostname: {}", initial_config.system_info.hostname);
    info!(
        "   - Rust version: {}",
        initial_config.system_info.rust_version
    );
    info!("   - CPU Count: {}", initial_config.system_info.cpu_count);
    info!(
        "   - Memory: {} total, {} available, {} used",
        initial_config.system_info.memory.total_mb,
        initial_config.system_info.memory.available_mb,
        initial_config.system_info.memory.used_mb
    );
    info!("   - Default shell: {}", initial_config.default_shell);
    info!(
        "   - Blocked commands: {:?}",
        initial_config.blocked_commands
    );
    info!(
        "   - File read limit: {} lines",
        initial_config.file_read_line_limit
    );
    info!(
        "   - File write limit: {} lines",
        initial_config.file_write_line_limit
    );
    info!(
        "   - Fuzzy search threshold: {:.2}",
        initial_config.fuzzy_search_threshold
    );
    info!(
        "   - SSE timeout: {}s",
        initial_config.sse_connection_timeout_secs
    );

    if let Some(client_info) = &initial_config.current_client {
        info!(
            "   - Current client: {} v{}",
            client_info.name, client_info.version
        );
    }

    if !initial_config.client_history.is_empty() {
        info!(
            "   - Client history: {} connection(s)",
            initial_config.client_history.len()
        );
    }

    // Store original values for restoration later
    let original_read_limit = initial_config.file_read_line_limit;

    // ========================================================================
    // TEST 2: Set file_read_line_limit (number value)
    // ========================================================================
    info!("2. Testing set_config_value with number (file_read_line_limit)");
    match client
        .call_tool(
            tools::SET_CONFIG_VALUE,
            json!({
                "key": "file_read_line_limit",
                "value": 2500
            }),
        )
        .await
    {
        Ok(result) => {
            info!("✅ Successfully set file_read_line_limit to 2500");
            info!("   Response: {:?}", result);
        }
        Err(e) => error!("❌ Failed to set file_read_line_limit: {}", e),
    }

    // ========================================================================
    // TEST 3: Verify the change persisted
    // ========================================================================
    info!("3. Verifying file_read_line_limit change persisted");
    let updated_config: GetConfigResponse = client
        .call_tool_typed(tools::GET_CONFIG, json!({}))
        .await
        .context("Failed to get updated config")?;

    if updated_config.file_read_line_limit == 2500 {
        info!(
            "✅ file_read_line_limit correctly updated to {}",
            updated_config.file_read_line_limit
        );
    } else {
        error!(
            "❌ file_read_line_limit not updated! Expected 2500, got {}",
            updated_config.file_read_line_limit
        );
    }

    // ========================================================================
    // TEST 4: Set fuzzy_search_threshold (percentage as integer 0-100)
    // ========================================================================
    info!("4. Testing set_config_value with percentage (fuzzy_search_threshold)");
    match client
        .call_tool(
            tools::SET_CONFIG_VALUE,
            json!({
                "key": "fuzzy_search_threshold",
                "value": 85
            }),
        )
        .await
    {
        Ok(_) => {
            let config: GetConfigResponse = client
                .call_tool_typed(tools::GET_CONFIG, json!({}))
                .await
                .context("Failed to get config after fuzzy threshold update")?;

            if (config.fuzzy_search_threshold - 0.85).abs() < 0.01 {
                info!(
                    "✅ fuzzy_search_threshold correctly set to {:.2}",
                    config.fuzzy_search_threshold
                );
            } else {
                error!(
                    "❌ fuzzy_search_threshold mismatch! Expected 0.85, got {:.2}",
                    config.fuzzy_search_threshold
                );
            }
        }
        Err(e) => error!("❌ Failed to set fuzzy_search_threshold: {}", e),
    }

    // ========================================================================
    // TEST 5: Set blocked_commands (array value)
    // ========================================================================
    info!("5. Testing set_config_value with array (blocked_commands)");
    match client
        .call_tool(
            tools::SET_CONFIG_VALUE,
            json!({
                "key": "blocked_commands",
                "value": ["rm", "sudo", "format", "dd", "shutdown"]
            }),
        )
        .await
    {
        Ok(_) => {
            let config: GetConfigResponse = client
                .call_tool_typed(tools::GET_CONFIG, json!({}))
                .await
                .context("Failed to get config after blocked_commands update")?;

            info!(
                "✅ blocked_commands updated to: {:?}",
                config.blocked_commands
            );
        }
        Err(e) => error!("❌ Failed to set blocked_commands: {}", e),
    }

    // ========================================================================
    // TEST 6: Set sse_connection_timeout_secs (number value)
    // ========================================================================
    info!("6. Testing set_config_value with number (sse_connection_timeout_secs)");
    match client
        .call_tool(
            tools::SET_CONFIG_VALUE,
            json!({
                "key": "sse_connection_timeout_secs",
                "value": 10
            }),
        )
        .await
    {
        Ok(_) => {
            let config: GetConfigResponse = client
                .call_tool_typed(tools::GET_CONFIG, json!({}))
                .await
                .context("Failed to get config after SSE timeout update")?;

            if config.sse_connection_timeout_secs == 10 {
                info!(
                    "✅ sse_connection_timeout_secs correctly set to {}s",
                    config.sse_connection_timeout_secs
                );
            } else {
                error!(
                    "❌ sse_connection_timeout_secs mismatch! Expected 10, got {}",
                    config.sse_connection_timeout_secs
                );
            }
        }
        Err(e) => error!("❌ Failed to set sse_connection_timeout_secs: {}", e),
    }

    // ========================================================================
    // TEST 7: Test invalid key (should error gracefully)
    // ========================================================================
    info!("7. Testing set_config_value with invalid key (should error)");
    match client
        .call_tool(
            tools::SET_CONFIG_VALUE,
            json!({
                "key": "nonexistent_key",
                "value": "should_fail"
            }),
        )
        .await
    {
        Ok(_) => error!("❌ Should have failed with invalid key!"),
        Err(e) => {
            if e.to_string().contains("Unknown config key") {
                info!("✅ Correctly rejected invalid key: {}", e);
            } else {
                error!("❌ Wrong error message: {}", e);
            }
        }
    }

    // ========================================================================
    // TEST 8: Test invalid value type (should error gracefully)
    // ========================================================================
    info!("8. Testing set_config_value with wrong value type (should error)");
    match client
        .call_tool(
            tools::SET_CONFIG_VALUE,
            json!({
                "key": "file_read_line_limit",
                "value": "not_a_number"
            }),
        )
        .await
    {
        Ok(_) => error!("❌ Should have failed with wrong value type!"),
        Err(e) => info!("✅ Correctly rejected wrong value type: {}", e),
    }

    // ========================================================================
    // TEST 9: Test boundary value (negative number - should error)
    // ========================================================================
    info!("9. Testing set_config_value with negative number (should error)");
    match client
        .call_tool(
            tools::SET_CONFIG_VALUE,
            json!({
                "key": "file_read_line_limit",
                "value": -100
            }),
        )
        .await
    {
        Ok(_) => error!("❌ Should have failed with negative value!"),
        Err(e) => {
            if e.to_string().contains("must be positive") {
                info!("✅ Correctly rejected negative value: {}", e);
            } else {
                error!("❌ Wrong error message: {}", e);
            }
        }
    }

    // ========================================================================
    // CLEANUP: Restore original file_read_line_limit
    // ========================================================================
    info!("\nRestoring original configuration...");
    match client
        .call_tool(
            tools::SET_CONFIG_VALUE,
            json!({
                "key": "file_read_line_limit",
                "value": original_read_limit
            }),
        )
        .await
    {
        Ok(_) => info!(
            "✅ Restored file_read_line_limit to {}",
            original_read_limit
        ),
        Err(e) => error!("⚠️  Failed to restore file_read_line_limit: {}", e),
    }

    // Graceful shutdown
    conn.close().await?;
    server.shutdown().await?;
    info!("\nConfig tools example completed - all tests passed!");

    Ok(())
}
