mod common;

use anyhow::Context;
use kodegen_mcp_client::responses::StartSearchResponse;
use kodegen_mcp_client::tools;
use serde_json::json;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting filesystem tools example");

    // Connect to kodegen server with filesystem category
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Filesystem])
    ).await?;

    info!("Connected to server: {:?}", client.server_info());

    // Run example with cleanup
    let result = run_filesystem_example(&client).await;

    // Always close client, regardless of example result
    client.close().await?;

    // Propagate any error from the example
    result
}

async fn run_filesystem_example(client: &kodegen_mcp_client::KodegenClient) -> anyhow::Result<()> {
    // Create a temporary test directory
    let test_dir = std::env::temp_dir().join("kodegen_test");
    let test_file = test_dir.join("test.txt");
    
    info!("Using test directory: {}", test_dir.display());

    // Run tests
    let test_result = async {

    // 1. CREATE_DIRECTORY - Create test directory
    info!("1. Testing create_directory");
    client.call_tool(
        tools::CREATE_DIRECTORY,
        json!({ "path": test_dir.to_string_lossy() })
    )
    .await
    .context("Failed to create test directory")?;
    info!("✅ Created test directory");

    // 2. WRITE_FILE - Write initial content
    info!("2. Testing write_file");
    client.call_tool(
        tools::WRITE_FILE,
        json!({
            "path": test_file.to_string_lossy(),
            "content": "Hello, kodegen!\nThis is a test file.\nLine 3",
            "mode": "rewrite"
        })
    )
    .await
    .context("Failed to write initial file content")?;
    info!("✅ Wrote initial file content");

    // 3. READ_FILE - Read the file back
    info!("3. Testing read_file");
    client.call_tool(
        tools::READ_FILE,
        json!({ "path": test_file.to_string_lossy() })
    )
    .await
    .context("Failed to read file")?;
    info!("✅ Read file successfully");

    // 4. GET_FILE_INFO - Get metadata
    info!("4. Testing get_file_info");
    client.call_tool(
        tools::GET_FILE_INFO,
        json!({ "path": test_file.to_string_lossy() })
    )
    .await
    .context("Failed to get file info")?;
    info!("✅ Got file info successfully");

    // 5. EDIT_BLOCK - Edit the file
    info!("5. Testing edit_block");
    client.call_tool(
        tools::EDIT_BLOCK,
        json!({
            "file_path": test_file.to_string_lossy(),
            "old_string": "test file",
            "new_string": "modified file"
        })
    )
    .await
    .context("Failed to edit file")?;
    info!("✅ Edited file successfully");

    // 6. LIST_DIRECTORY - List directory contents
    info!("6. Testing list_directory");
    client.call_tool(
        tools::LIST_DIRECTORY,
        json!({ "path": test_dir.to_string_lossy(), "depth": 1 })
    )
    .await
    .context("Failed to list directory")?;
    info!("✅ Listed directory successfully");

    // 7. MOVE_FILE - Rename the file
    let test_file_renamed = test_dir.join("test_renamed.txt");
    info!("7. Testing move_file");
    client.call_tool(
        tools::MOVE_FILE,
        json!({
            "source": test_file.to_string_lossy(),
            "destination": test_file_renamed.to_string_lossy()
        })
    )
    .await
    .context("Failed to move file")?;
    info!("✅ Moved file successfully");

    // 8. READ_MULTIPLE_FILES - Read multiple files
    let test_file2 = test_dir.join("test2.txt");
    client.call_tool(
        tools::WRITE_FILE,
        json!({
            "path": test_file2.to_string_lossy(),
            "content": "Second test file",
            "mode": "rewrite"
        })
    )
    .await
    .context("Failed to create test file 2")?;
    info!("✅ Created test file 2");

    info!("8. Testing read_multiple_files");
    client.call_tool(
        tools::READ_MULTIPLE_FILES,
        json!({
            "paths": [
                test_file_renamed.to_string_lossy(),
                test_file2.to_string_lossy()
            ]
        })
    )
    .await
    .context("Failed to read multiple files")?;
    info!("✅ Read multiple files successfully");

    // 9. START_SEARCH - Start file content search
    info!("9. Testing start_search");
    let response: StartSearchResponse = client.call_tool_typed(
        tools::START_SEARCH,
        json!({
            "path": test_dir.to_string_lossy(),
            "pattern": "test",
            "search_type": "content",
            "timeout_ms": 5000
        })
    ).await?;
    
    let session_id = response.session_id;
    info!("Started search with session ID: {}", session_id);

    // 10. GET_MORE_SEARCH_RESULTS - Get search results
    {
        info!("10. Testing get_more_search_results");
        client.call_tool(
            tools::GET_MORE_SEARCH_RESULTS,
            json!({ "session_id": session_id, "offset": 0, "length": 10 })
        )
        .await
        .context("Failed to get search results")?;
        info!("✅ Got search results successfully");

        // 11. LIST_SEARCHES - List active searches
        info!("11. Testing list_searches");
        client.call_tool(tools::LIST_SEARCHES, json!({}))
            .await
            .context("Failed to list searches")?;
        info!("✅ Listed searches successfully");

        // 12. STOP_SEARCH - Stop the search
        info!("12. Testing stop_search");
        client.call_tool(
            tools::STOP_SEARCH,
            json!({ "session_id": session_id })
        )
        .await
        .context("Failed to stop search")?;
        info!("✅ Stopped search successfully");
    }

    // 13. DELETE_FILE - Delete test files
    info!("13. Testing delete_file");
    for file in [&test_file_renamed, &test_file2] {
        client.call_tool(
            tools::DELETE_FILE,
            json!({ "path": file.to_string_lossy() })
        )
        .await
        .context("Failed to delete file")?;
        info!("✅ Deleted file: {}", file.display());
    }

    // 14. DELETE_DIRECTORY - Clean up test directory (as part of test)
    info!("14. Testing delete_directory");
    client.call_tool(
        tools::DELETE_DIRECTORY,
        json!({ "path": test_dir.to_string_lossy() })
    )
    .await
    .context("Failed to delete directory")?;
    info!("✅ Deleted directory successfully");

    info!("Filesystem tools example tests completed");
    Ok::<(), anyhow::Error>(())
    }.await;

    // Always cleanup test directory, regardless of test result
    cleanup_filesystem_resources(client, &test_dir).await;

    // Propagate test result
    test_result
}

async fn cleanup_filesystem_resources(client: &kodegen_mcp_client::KodegenClient, test_dir: &std::path::Path) {
    use tracing::error;
    
    info!("\nCleaning up test directory...");
    
    // Try to delete using the filesystem tool first
    if let Err(e) = client.call_tool(
        tools::DELETE_DIRECTORY,
        json!({ "path": test_dir.to_string_lossy() })
    ).await {
        error!("⚠️  Failed to delete directory via tool {}: {}", test_dir.display(), e);
        
        // Fall back to direct filesystem removal
        if let Err(e) = std::fs::remove_dir_all(test_dir) {
            error!("⚠️  Failed to remove directory {}: {}", test_dir.display(), e);
            error!("   Manual cleanup required: rm -rf {}", test_dir.display());
        } else {
            info!("✅ Cleaned up test directory via fallback: {}", test_dir.display());
        }
    } else {
        info!("✅ Cleaned up test directory: {}", test_dir.display());
    }
}
