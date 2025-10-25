//! Direct memory system test - bypasses MCP layer for debugging
//!
//! Uses MemoryCoordinator directly to test memory operations

use anyhow::{Context, Result, anyhow};
use kodegen_candle_agent::capability::registry::{FromRegistry, TextEmbeddingModel};
use kodegen_candle_agent::memory::core::manager::coordinator::MemoryCoordinator;
use kodegen_candle_agent::memory::core::manager::surreal::SurrealDBMemoryManager;
use kodegen_candle_agent::memory::core::primitives::metadata::MemoryMetadata;
use kodegen_candle_agent::memory::core::ops::filter::MemoryFilter;
use kodegen_candle_agent::domain::memory::primitives::types::MemoryTypeEnum;
use surrealdb::engine::any::connect;
use std::collections::HashSet;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("========================================");
    println!("  Direct Memory System Test");
    println!("========================================\n");

    // Initialize memory coordinator (same as main.rs)
    let coordinator = initialize_memory_coordinator().await?;

    println!("✅ Memory coordinator initialized\n");

    // Run memory tests
    run_memory_tests(&coordinator).await?;

    println!("\n========================================");
    println!("  All tests completed successfully!");
    println!("========================================");

    Ok(())
}

async fn initialize_memory_coordinator() -> Result<Arc<MemoryCoordinator>> {
    println!("Initializing memory coordinator...");

    // Get embedding model from registry (Stella 400M variant - registered by default)
    let emb_model = TextEmbeddingModel::from_registry("dunzhang/stella_en_400M_v5")
        .ok_or_else(|| anyhow!("Stella embedding model not found in registry"))?;
    println!("  ✓ Loaded embedding model from registry");

    // Database path setup
    let db_path = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("kodegen")
        .join("candle-agent-test.db");

    // Ensure database directory exists
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("Failed to create database directory")?;
    }
    println!("  ✓ Database path: {}", db_path.display());

    let db_url = format!("surrealkv://{}", db_path.display());

    // Connect to database
    let db = connect(&db_url)
        .await
        .context("Failed to connect to database")?;
    println!("  ✓ Connected to SurrealDB");

    // Initialize database namespace
    db.use_ns("kodegen")
        .use_db("candle_agent_test")
        .await
        .context("Failed to initialize database namespace")?;
    println!("  ✓ Database namespace initialized");

    // Create SurrealDBMemoryManager with embedding model
    let surreal_manager = SurrealDBMemoryManager::with_embedding_model(db, emb_model.clone());

    // Initialize database tables and schema
    surreal_manager
        .initialize()
        .await
        .map_err(|e| anyhow!("Failed to initialize memory tables: {:?}", e))?;
    println!("  ✓ Database tables initialized");

    let surreal_arc = Arc::new(surreal_manager);

    // Create MemoryCoordinator
    let coordinator = MemoryCoordinator::new(surreal_arc, emb_model)
        .await
        .map_err(|e| anyhow!("Failed to create memory coordinator: {:?}", e))?;
    println!("  ✓ Memory coordinator created");

    Ok(Arc::new(coordinator))
}

async fn run_memory_tests(coordinator: &Arc<MemoryCoordinator>) -> Result<()> {
    // ========================================================================
    // PHASE 1: Create memories in two libraries
    // ========================================================================
    println!("PHASE 1: Creating memories in two libraries\n");

    // Library 1: rust_patterns
    println!("1. Storing Rust pattern #1");
    let mut metadata1 = MemoryMetadata::default();
    metadata1.add_tag("rust_patterns");
    let mem1 = coordinator
        .add_memory(
            "Error handling pattern using Result<T, E> with the ? operator for clean propagation".to_string(),
            MemoryTypeEnum::LongTerm,
            Some(metadata1),
        )
        .await?;
    println!("   ✅ Created memory: {} in 'rust_patterns'", mem1.id());

    println!("2. Storing Rust pattern #2");
    let mut metadata2 = MemoryMetadata::default();
    metadata2.add_tag("rust_patterns");
    let mem2 = coordinator
        .add_memory(
            "Async/await pattern for file I/O operations using tokio::fs with proper error handling".to_string(),
            MemoryTypeEnum::LongTerm,
            Some(metadata2),
        )
        .await?;
    println!("   ✅ Created memory: {} in 'rust_patterns'", mem2.id());

    // Library 2: debugging_insights
    println!("3. Storing debugging insight #1");
    let mut metadata3 = MemoryMetadata::default();
    metadata3.add_tag("debugging_insights");
    let mem3 = coordinator
        .add_memory(
            "React re-renders happen when props or state change - use React.memo to prevent unnecessary renders".to_string(),
            MemoryTypeEnum::LongTerm,
            Some(metadata3),
        )
        .await?;
    println!("   ✅ Created memory: {} in 'debugging_insights'", mem3.id());

    println!("4. Storing debugging insight #2");
    let mut metadata4 = MemoryMetadata::default();
    metadata4.add_tag("debugging_insights");
    let mem4 = coordinator
        .add_memory(
            "SQL N+1 query problem - use eager loading with JOIN instead of lazy loading to reduce DB calls".to_string(),
            MemoryTypeEnum::LongTerm,
            Some(metadata4),
        )
        .await?;
    println!("   ✅ Created memory: {} in 'debugging_insights'", mem4.id());

    // ========================================================================
    // PHASE 2: List all libraries
    // ========================================================================
    println!("\nPHASE 2: Listing all memory libraries\n");

    println!("5. Getting all memories and extracting unique tags");
    let filter = MemoryFilter::new();
    let all_memories = coordinator.get_memories(filter).await?;

    let mut libraries: HashSet<String> = HashSet::new();
    for memory in &all_memories {
        for tag in &memory.metadata.tags {
            libraries.insert(tag.to_string());
        }
    }

    let mut libraries_vec: Vec<String> = libraries.into_iter().collect();
    libraries_vec.sort();

    println!("   ✅ Found {} libraries:", libraries_vec.len());
    for lib in &libraries_vec {
        println!("      - {}", lib);
    }

    // ========================================================================
    // PHASE 3: Recall from each library (semantic search)
    // ========================================================================
    println!("\nPHASE 3: Recalling memories using semantic search\n");

    println!("6. Recalling from 'rust_patterns' (context: 'error handling')");
    let filter1 = MemoryFilter::new().with_tags(vec!["rust_patterns".to_string()]);
    let results1 = coordinator
        .search_memories("error handling", 5, Some(filter1))
        .await?;

    println!("   ✅ Found {} memories in 'rust_patterns':", results1.len());
    for memory in &results1 {
        println!("      - ID: {}", memory.id());
        println!("        Relevance: {:.3}", memory.metadata.importance);
        println!("        Content: {}", memory.content().to_string());
    }

    println!("\n7. Recalling from 'debugging_insights' (context: 'performance optimization')");
    let filter2 = MemoryFilter::new().with_tags(vec!["debugging_insights".to_string()]);
    let results2 = coordinator
        .search_memories("performance optimization", 5, Some(filter2))
        .await?;

    println!("   ✅ Found {} memories in 'debugging_insights':", results2.len());
    for memory in &results2 {
        println!("      - ID: {}", memory.id());
        println!("        Relevance: {:.3}", memory.metadata.importance);
        println!("        Content: {}", memory.content().to_string());
    }

    // ========================================================================
    // PHASE 4: Deduplication test
    // ========================================================================
    println!("\nPHASE 4: Testing deduplication (duplicate content detection)\n");

    let duplicate_content = "This is a duplicate test string for deduplication verification";

    println!("8. Storing duplicate content in 'rust_patterns' (first time)");
    let mut dup_meta1 = MemoryMetadata::default();
    dup_meta1.add_tag("rust_patterns");
    let dup1 = coordinator
        .add_memory(
            duplicate_content.to_string(),
            MemoryTypeEnum::LongTerm,
            Some(dup_meta1),
        )
        .await?;
    println!("   ✅ First insertion - Memory ID: {}", dup1.id());

    println!("9. Storing SAME content in 'rust_patterns' (second time - should deduplicate)");
    let mut dup_meta2 = MemoryMetadata::default();
    dup_meta2.add_tag("rust_patterns");
    let dup2 = coordinator
        .add_memory(
            duplicate_content.to_string(),
            MemoryTypeEnum::LongTerm,
            Some(dup_meta2),
        )
        .await?;
    println!("   ✅ Second insertion - Memory ID: {}", dup2.id());

    if dup1.id() == dup2.id() {
        println!("\n   🎉 DEDUPLICATION VERIFIED:");
        println!("      Same memory_id returned: {}", dup1.id());
        println!("      Content hash matched - importance reset, but same entry preserved!");
    } else {
        println!("\n   ⚠️  Different memory IDs - deduplication may not have worked");
        println!("      First:  {}", dup1.id());
        println!("      Second: {}", dup2.id());
    }

    println!("\n10. Storing SAME content in 'debugging_insights' (different library)");
    let mut dup_meta3 = MemoryMetadata::default();
    dup_meta3.add_tag("debugging_insights");
    let dup3 = coordinator
        .add_memory(
            duplicate_content.to_string(),
            MemoryTypeEnum::LongTerm,
            Some(dup_meta3),
        )
        .await?;
    println!("   ✅ Third insertion (different library) - Memory ID: {}", dup3.id());

    if dup1.id() == dup3.id() {
        println!("\n   🎉 CROSS-LIBRARY DEDUPLICATION VERIFIED:");
        println!("      Same memory_id across libraries: {}", dup1.id());
        println!("      Content hash is global - same content = same memory entry!");
    } else {
        println!("\n   ℹ️  Different memory ID in different library");
        println!("      First (rust_patterns):      {}", dup1.id());
        println!("      Third (debugging_insights): {}", dup3.id());
    }

    Ok(())
}
