mod common;

use anyhow::Context;
use serde_json::json;
use tracing::info;

#[derive(serde::Deserialize, Debug)]
struct MemorizeResponse {
    success: bool,
    memory_id: String,
    library: String,
    message: String,
}

#[derive(serde::Deserialize, Debug)]
struct ListLibrariesResponse {
    libraries: Vec<String>,
    count: usize,
}

#[derive(serde::Deserialize, Debug)]
struct Memory {
    id: String,
    content: String,
    created_at: String,
    relevance_score: f64,
}

#[derive(serde::Deserialize, Debug)]
struct RecallResponse {
    memories: Vec<Memory>,
    library: String,
    count: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting candle-agent memory tools example");

    // Connect to kodegen-candle-agent server
    let (conn, mut server) = common::connect_to_local_sse_server().await?;

    // Wrap client with logging
    let workspace_root = common::find_workspace_root()
        .context("Failed to find workspace root")?;
    let log_path = workspace_root.join("tmp/mcp-client/memory.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // Run example with cleanup
    let result = run_memory_example(&client).await;

    // Always close connection, regardless of example result
    conn.close().await?;
    server.shutdown().await?;

    // Propagate any error from the example
    result
}

async fn run_memory_example(client: &common::LoggingClient) -> anyhow::Result<()> {
    info!("========================================");
    info!("  Memory Tools Demonstration");
    info!("========================================\n");

    // ========================================================================
    // PHASE 1: Create memories in two different libraries
    // ========================================================================
    info!("PHASE 1: Creating memories in two libraries\n");

    // Library 1: rust_patterns
    info!("1. Storing Rust pattern #1");
    let mem1: MemorizeResponse = client
        .call_tool_typed(
            "memorize",
            json!({
                "library": "rust_patterns",
                "content": "Error handling pattern using Result<T, E> with the ? operator for clean propagation"
            }),
        )
        .await
        .context("Failed to memorize rust pattern #1")?;
    info!("   ✅ Created memory: {} in '{}'", mem1.memory_id, mem1.library);

    info!("2. Storing Rust pattern #2");
    let mem2: MemorizeResponse = client
        .call_tool_typed(
            "memorize",
            json!({
                "library": "rust_patterns",
                "content": "Async/await pattern for file I/O operations using tokio::fs with proper error handling"
            }),
        )
        .await
        .context("Failed to memorize rust pattern #2")?;
    info!("   ✅ Created memory: {} in '{}'", mem2.memory_id, mem2.library);

    // Library 2: debugging_insights
    info!("3. Storing debugging insight #1");
    let mem3: MemorizeResponse = client
        .call_tool_typed(
            "memorize",
            json!({
                "library": "debugging_insights",
                "content": "React re-renders happen when props or state change - use React.memo to prevent unnecessary renders"
            }),
        )
        .await
        .context("Failed to memorize debugging insight #1")?;
    info!("   ✅ Created memory: {} in '{}'", mem3.memory_id, mem3.library);

    info!("4. Storing debugging insight #2");
    let mem4: MemorizeResponse = client
        .call_tool_typed(
            "memorize",
            json!({
                "library": "debugging_insights",
                "content": "SQL N+1 query problem - use eager loading with JOIN instead of lazy loading to reduce DB calls"
            }),
        )
        .await
        .context("Failed to memorize debugging insight #2")?;
    info!("   ✅ Created memory: {} in '{}'", mem4.memory_id, mem4.library);

    // ========================================================================
    // PHASE 2: List all libraries
    // ========================================================================
    info!("\nPHASE 2: Listing all memory libraries\n");

    info!("5. Calling list_memory_libraries()");
    let libraries: ListLibrariesResponse = client
        .call_tool_typed("list_memory_libraries", json!({}))
        .await
        .context("Failed to list memory libraries")?;

    info!("   ✅ Found {} libraries:", libraries.count);
    for lib in &libraries.libraries {
        info!("      - {}", lib);
    }

    // ========================================================================
    // PHASE 3: Recall from each library (semantic search)
    // ========================================================================
    info!("\nPHASE 3: Recalling memories using semantic search\n");

    info!("6. Recalling from 'rust_patterns' (context: 'error handling')");
    let recall1: RecallResponse = client
        .call_tool_typed(
            "recall",
            json!({
                "library": "rust_patterns",
                "context": "error handling",
                "limit": 5
            }),
        )
        .await
        .context("Failed to recall from rust_patterns")?;

    info!("   ✅ Found {} memories in '{}':", recall1.count, recall1.library);
    for memory in &recall1.memories {
        info!("      - ID: {}", memory.id);
        info!("        Relevance: {:.3}", memory.relevance_score);
        info!("        Content: {}", memory.content);
    }

    info!("\n7. Recalling from 'debugging_insights' (context: 'performance optimization')");
    let recall2: RecallResponse = client
        .call_tool_typed(
            "recall",
            json!({
                "library": "debugging_insights",
                "context": "performance optimization",
                "limit": 5
            }),
        )
        .await
        .context("Failed to recall from debugging_insights")?;

    info!("   ✅ Found {} memories in '{}':", recall2.count, recall2.library);
    for memory in &recall2.memories {
        info!("      - ID: {}", memory.id);
        info!("        Relevance: {:.3}", memory.relevance_score);
        info!("        Content: {}", memory.content);
    }

    // ========================================================================
    // PHASE 4: Deduplication test - Store duplicate content
    // ========================================================================
    info!("\nPHASE 4: Testing deduplication (duplicate content detection)\n");

    let duplicate_content = "This is a duplicate test string for deduplication verification";

    info!("8. Storing duplicate content in 'rust_patterns' (first time)");
    let dup1: MemorizeResponse = client
        .call_tool_typed(
            "memorize",
            json!({
                "library": "rust_patterns",
                "content": duplicate_content
            }),
        )
        .await
        .context("Failed to memorize duplicate #1")?;
    info!("   ✅ First insertion - Memory ID: {}", dup1.memory_id);

    info!("9. Storing SAME content in 'rust_patterns' (second time - should deduplicate)");
    let dup2: MemorizeResponse = client
        .call_tool_typed(
            "memorize",
            json!({
                "library": "rust_patterns",
                "content": duplicate_content
            }),
        )
        .await
        .context("Failed to memorize duplicate #2")?;
    info!("   ✅ Second insertion - Memory ID: {}", dup2.memory_id);

    if dup1.memory_id == dup2.memory_id {
        info!("\n   🎉 DEDUPLICATION VERIFIED:");
        info!("      Same memory_id returned: {}", dup1.memory_id);
        info!("      Content hash matched - importance reset, but same entry preserved!");
    } else {
        info!("\n   ⚠️  Different memory IDs - deduplication may not have worked");
        info!("      First:  {}", dup1.memory_id);
        info!("      Second: {}", dup2.memory_id);
    }

    info!("\n10. Storing SAME content in 'debugging_insights' (different library)");
    let dup3: MemorizeResponse = client
        .call_tool_typed(
            "memorize",
            json!({
                "library": "debugging_insights",
                "content": duplicate_content
            }),
        )
        .await
        .context("Failed to memorize duplicate #3")?;
    info!("   ✅ Third insertion (different library) - Memory ID: {}", dup3.memory_id);

    if dup1.memory_id == dup3.memory_id {
        info!("\n   🎉 CROSS-LIBRARY DEDUPLICATION VERIFIED:");
        info!("      Same memory_id across libraries: {}", dup1.memory_id);
        info!("      Content hash is global - same content = same memory entry!");
    } else {
        info!("\n   ℹ️  Different memory ID in different library");
        info!("      First (rust_patterns):      {}", dup1.memory_id);
        info!("      Third (debugging_insights): {}", dup3.memory_id);
    }

    // ========================================================================
    // Final summary
    // ========================================================================
    info!("\n========================================");
    info!("  Example completed successfully!");
    info!("========================================");
    info!("Demonstrated features:");
    info!("  ✅ Memorize content with automatic embeddings");
    info!("  ✅ List all memory libraries");
    info!("  ✅ Recall using semantic similarity search");
    info!("  ✅ Deduplication via content hash");
    info!("  ✅ Multi-library organization");

    Ok(())
}
