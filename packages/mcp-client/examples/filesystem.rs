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

    info!("Starting filesystem tools example");

    // Connect to kodegen server with filesystem category
    let client = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::Filesystem])
    ).await?;

    info!("Connected to server: {:?}", client.server_info());

    // Create a temporary test directory
    let test_dir = std::env::temp_dir().join("kodegen_test");
    let test_file = test_dir.join("test.txt");
    
    info!("Using test directory: {}", test_dir.display());

    // 1. CREATE_DIRECTORY - Create test directory
    info!("1. Testing create_directory");
    match client.call_tool(
        tools::CREATE_DIRECTORY,
        json!({ "path": test_dir.to_string_lossy() })
    ).await {
        Ok(result) => info!("Created directory: {:?}", result),
        Err(e) => error!("Failed to create directory: {}", e),
    }

    // 2. WRITE_FILE - Write initial content
    info!("2. Testing write_file");
    match client.call_tool(
        tools::WRITE_FILE,
        json!({
            "path": test_file.to_string_lossy(),
            "content": "Hello, kodegen!\nThis is a test file.\nLine 3",
            "mode": "rewrite"
        })
    ).await {
        Ok(result) => info!("Wrote file: {:?}", result),
        Err(e) => error!("Failed to write file: {}", e),
    }

    // 3. READ_FILE - Read the file back
    info!("3. Testing read_file");
    match client.call_tool(
        tools::READ_FILE,
        json!({ "path": test_file.to_string_lossy() })
    ).await {
        Ok(result) => info!("Read file: {:?}", result),
        Err(e) => error!("Failed to read file: {}", e),
    }

    // 4. GET_FILE_INFO - Get metadata
    info!("4. Testing get_file_info");
    match client.call_tool(
        tools::GET_FILE_INFO,
        json!({ "path": test_file.to_string_lossy() })
    ).await {
        Ok(result) => info!("File info: {:?}", result),
        Err(e) => error!("Failed to get file info: {}", e),
    }

    // 5. EDIT_BLOCK - Edit the file
    info!("5. Testing edit_block");
    match client.call_tool(
        tools::EDIT_BLOCK,
        json!({
            "file_path": test_file.to_string_lossy(),
            "old_string": "test file",
            "new_string": "modified file"
        })
    ).await {
        Ok(result) => info!("Edited file: {:?}", result),
        Err(e) => error!("Failed to edit file: {}", e),
    }

    // 6. LIST_DIRECTORY - List directory contents
    info!("6. Testing list_directory");
    match client.call_tool(
        tools::LIST_DIRECTORY,
        json!({ "path": test_dir.to_string_lossy(), "depth": 1 })
    ).await {
        Ok(result) => info!("Directory contents: {:?}", result),
        Err(e) => error!("Failed to list directory: {}", e),
    }

    // 7. MOVE_FILE - Rename the file
    let test_file_renamed = test_dir.join("test_renamed.txt");
    info!("7. Testing move_file");
    match client.call_tool(
        tools::MOVE_FILE,
        json!({
            "source": test_file.to_string_lossy(),
            "destination": test_file_renamed.to_string_lossy()
        })
    ).await {
        Ok(result) => info!("Moved file: {:?}", result),
        Err(e) => error!("Failed to move file: {}", e),
    }

    // 8. READ_MULTIPLE_FILES - Read multiple files
    let test_file2 = test_dir.join("test2.txt");
    match client.call_tool(
        tools::WRITE_FILE,
        json!({
            "path": test_file2.to_string_lossy(),
            "content": "Second test file",
            "mode": "rewrite"
        })
    ).await {
        Ok(result) => info!("Created test file 2: {:?}", result),
        Err(e) => error!("Failed to create test file 2: {}", e),
    }

    info!("8. Testing read_multiple_files");
    match client.call_tool(
        tools::READ_MULTIPLE_FILES,
        json!({
            "paths": [
                test_file_renamed.to_string_lossy(),
                test_file2.to_string_lossy()
            ]
        })
    ).await {
        Ok(result) => info!("Read multiple files: {:?}", result),
        Err(e) => error!("Failed to read multiple files: {}", e),
    }

    // 9. START_SEARCH - Start file content search
    info!("9. Testing start_search");
    let search_result = client.call_tool(
        tools::START_SEARCH,
        json!({
            "path": test_dir.to_string_lossy(),
            "pattern": "test",
            "search_type": "content",
            "timeout_ms": 5000
        })
    ).await;
    
    let session_id = if let Ok(result) = &search_result {
        info!("Started search: {:?}", result);
        // Parse session_id from CallToolResult content
        result.content.first()
            .and_then(|content| content.as_text())
            .and_then(|text_content| {
                serde_json::from_str::<serde_json::Value>(&text_content.text)
                    .ok()
                    .and_then(|v| v["session_id"].as_str().map(String::from))
            })
    } else {
        error!("Failed to start search: {:?}", search_result);
        None
    };

    // 10. GET_MORE_SEARCH_RESULTS - Get search results
    if let Some(sid) = session_id {
        info!("10. Testing get_more_search_results");
        match client.call_tool(
            tools::GET_MORE_SEARCH_RESULTS,
            json!({ "session_id": sid, "offset": 0, "length": 10 })
        ).await {
            Ok(result) => info!("Search results: {:?}", result),
            Err(e) => error!("Failed to get search results: {}", e),
        }

        // 11. LIST_SEARCHES - List active searches
        info!("11. Testing list_searches");
        match client.call_tool(tools::LIST_SEARCHES, json!({})).await {
            Ok(result) => info!("Active searches: {:?}", result),
            Err(e) => error!("Failed to list searches: {}", e),
        }

        // 12. STOP_SEARCH - Stop the search
        info!("12. Testing stop_search");
        match client.call_tool(
            tools::STOP_SEARCH,
            json!({ "session_id": sid })
        ).await {
            Ok(result) => info!("Stopped search: {:?}", result),
            Err(e) => error!("Failed to stop search: {}", e),
        }
    }

    // 13. DELETE_FILE - Delete test files
    info!("13. Testing delete_file");
    for file in [&test_file_renamed, &test_file2] {
        match client.call_tool(
            tools::DELETE_FILE,
            json!({ "path": file.to_string_lossy() })
        ).await {
            Ok(result) => info!("Deleted file: {:?}", result),
            Err(e) => error!("Failed to delete file: {}", e),
        }
    }

    // 14. DELETE_DIRECTORY - Clean up test directory
    info!("14. Testing delete_directory");
    match client.call_tool(
        tools::DELETE_DIRECTORY,
        json!({ "path": test_dir.to_string_lossy() })
    ).await {
        Ok(result) => info!("Deleted directory: {:?}", result),
        Err(e) => error!("Failed to delete directory: {}", e),
    }

    // Graceful shutdown
    client.close().await?;
    info!("Filesystem tools example completed successfully");

    Ok(())
}
