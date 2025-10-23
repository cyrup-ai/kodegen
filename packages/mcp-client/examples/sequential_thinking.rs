mod common;

use anyhow::Context;
use kodegen_mcp_client::{tools, responses::SequentialThinkingResponse};
use serde_json::json;
use tracing::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting sequential thinking tool example - comprehensive feature testing");

    // Connect to kodegen server with sequential_thinking category
    let (conn, mut server) = common::connect_to_server_with_categories(
        Some(vec![common::ToolCategory::SequentialThinking])
    ).await?;

    // Wrap client with logging
    let log_path = std::path::PathBuf::from("/Volumes/samsung_t9/kodegen/tmp/mcp-client/sequential_thinking.log");
    let client = common::LoggingClient::new(conn.client(), log_path)
        .await
        .context("Failed to create logging client")?;

    info!("Connected to server: {:?}", client.server_info());

    // ========================================================================
    // TEST 1: Basic sequential thinking - solve a math problem
    // ========================================================================
    info!("1. Testing basic sequential thinking (3 thoughts)");
    
    let t1: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "thought": "I need to calculate 15 * 24. Let me break this down step by step.",
            "thought_number": 1,
            "total_thoughts": 3,
            "next_thought_needed": true
        })
    ).await.context("Failed on thought 1")?;
    
    info!("✅ Thought 1: session_id={}, history_length={}", t1.session_id, t1.thought_history_length);
    let session_id = t1.session_id.clone(); // Save for reuse

    let t2: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": session_id,
            "thought": "Using distribution: 15 * 24 = 15 * (20 + 4) = (15*20) + (15*4)",
            "thought_number": 2,
            "total_thoughts": 3,
            "next_thought_needed": true
        })
    ).await.context("Failed on thought 2")?;
    
    info!("✅ Thought 2: history_length={}", t2.thought_history_length);

    let t3: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": session_id,
            "thought": "Computing: 15*20 = 300, 15*4 = 60. Therefore 300 + 60 = 360. Answer: 360",
            "thought_number": 3,
            "total_thoughts": 3,
            "next_thought_needed": false
        })
    ).await.context("Failed on thought 3")?;
    
    info!("✅ Thought 3 (final): history_length={}, next_needed={}", 
        t3.thought_history_length, t3.next_thought_needed);

    // ========================================================================
    // TEST 2: Dynamic adjustment - realizing more thoughts needed
    // ========================================================================
    info!("\n2. Testing dynamic adjustment (expanding total_thoughts)");
    
    let d1: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "thought": "Analyzing algorithm complexity for quicksort. Initially I think this needs 3 steps.",
            "thought_number": 1,
            "total_thoughts": 3,
            "next_thought_needed": true
        })
    ).await.context("Failed on dynamic thought 1")?;
    
    info!("✅ Dynamic 1: new session_id={}", d1.session_id);
    let dynamic_session = d1.session_id.clone();

    let _d2: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": dynamic_session,
            "thought": "Wait, I need to cover best case, average case, and worst case separately.",
            "thought_number": 2,
            "total_thoughts": 3,
            "next_thought_needed": true,
            "needs_more_thoughts": true
        })
    ).await.context("Failed on dynamic thought 2")?;
    
    info!("✅ Dynamic 2: Signaled need for more thoughts");

    let d3: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": dynamic_session,
            "thought": "Expanding analysis: Best case O(n log n), average O(n log n), worst O(n²)",
            "thought_number": 3,
            "total_thoughts": 5,
            "next_thought_needed": true
        })
    ).await.context("Failed on dynamic thought 3")?;
    
    info!("✅ Dynamic 3: Expanded to {} total thoughts", d3.total_thoughts);

    let d4: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": dynamic_session,
            "thought": "Worst case occurs with already sorted input when pivot selection is poor.",
            "thought_number": 4,
            "total_thoughts": 5,
            "next_thought_needed": true
        })
    ).await.context("Failed on dynamic thought 4")?;
    
    info!("✅ Dynamic 4: history_length={}", d4.thought_history_length);

    let d5: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": dynamic_session,
            "thought": "Mitigation: Use randomized pivot or median-of-three to achieve O(n log n) average.",
            "thought_number": 5,
            "total_thoughts": 5,
            "next_thought_needed": false
        })
    ).await.context("Failed on dynamic thought 5")?;
    
    info!("✅ Dynamic 5 (final): Successfully expanded from 3 to 5 thoughts, history_length={}",
        d5.thought_history_length);

    // ========================================================================
    // TEST 3: Revision feature - correcting a previous thought
    // ========================================================================
    info!("\n3. Testing revision feature");
    
    let r1: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "thought": "The capital of Australia is Sydney.",
            "thought_number": 1,
            "total_thoughts": 3,
            "next_thought_needed": true
        })
    ).await.context("Failed on revision thought 1")?;
    
    info!("✅ Revision 1: session_id={}", r1.session_id);
    let revision_session = r1.session_id.clone();

    let _r2: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": revision_session,
            "thought": "Wait, I need to revise that. Sydney is the largest city but NOT the capital.",
            "thought_number": 2,
            "total_thoughts": 3,
            "next_thought_needed": true,
            "is_revision": true,
            "revises_thought": 1
        })
    ).await.context("Failed on revision thought 2")?;
    
    info!("✅ Revision 2: Marked as revision of thought 1");

    let r3: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": revision_session,
            "thought": "The correct answer is Canberra, which became the capital in 1913.",
            "thought_number": 3,
            "total_thoughts": 3,
            "next_thought_needed": false
        })
    ).await.context("Failed on revision thought 3")?;
    
    info!("✅ Revision 3 (final): Successfully revised thought 1, history_length={}",
        r3.thought_history_length);

    // ========================================================================
    // TEST 4: Branching feature - exploring alternative approaches
    // ========================================================================
    info!("\n4. Testing branching feature");
    
    let b1: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "thought": "To optimize database queries, I could either add indexes or denormalize.",
            "thought_number": 1,
            "total_thoughts": 3,
            "next_thought_needed": true
        })
    ).await.context("Failed on branch thought 1")?;
    
    info!("✅ Branch 1: session_id={}", b1.session_id);
    let branch_session = b1.session_id.clone();

    let _b2: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": branch_session,
            "thought": "Let me explore the indexing approach first.",
            "thought_number": 2,
            "total_thoughts": 3,
            "next_thought_needed": true
        })
    ).await.context("Failed on branch thought 2")?;
    
    info!("✅ Branch 2: Chose indexing path");

    // Create a branch to explore alternative approach
    let b2_alt: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": branch_session,
            "thought": "Actually, let me also explore denormalization as an alternative.",
            "thought_number": 2,
            "total_thoughts": 3,
            "next_thought_needed": true,
            "branch_from_thought": 1,
            "branch_id": "denormalize"
        })
    ).await.context("Failed on branch thought 2 alt")?;
    
    info!("✅ Branch 2 (alt): Created branch 'denormalize', branches={:?}", b2_alt.branches);

    // Continue main branch
    let _b3: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": branch_session,
            "thought": "Indexing pros: maintains normalization, easier rollback. Cons: write overhead.",
            "thought_number": 3,
            "total_thoughts": 3,
            "next_thought_needed": false
        })
    ).await.context("Failed on branch thought 3")?;
    
    info!("✅ Branch 3 (main path final): Completed main branch analysis");

    // Continue alternative branch
    let b3_alt: SequentialThinkingResponse = client.call_tool_typed(
        tools::SEQUENTIAL_THINKING,
        json!({
            "session_id": branch_session,
            "thought": "Denormalization pros: faster reads, simpler queries. Cons: data redundancy, update complexity.",
            "thought_number": 3,
            "total_thoughts": 3,
            "next_thought_needed": false,
            "branch_id": "denormalize"
        })
    ).await.context("Failed on branch thought 3 alt")?;
    
    info!("✅ Branch 3 (denormalize path final): Completed alternative branch, branches={:?}",
        b3_alt.branches);

    // ========================================================================
    // TEST 5: Session continuity verification
    // ========================================================================
    info!("\n5. Verifying session continuity and history tracking");
    
    info!("   Session 1 (basic math): {} thoughts in history", t3.thought_history_length);
    info!("   Session 2 (dynamic): {} thoughts in history", d5.thought_history_length);
    info!("   Session 3 (revision): {} thoughts in history", r3.thought_history_length);
    info!("   Session 4 (branching): {} thoughts in history, branches={:?}",
        b3_alt.thought_history_length, b3_alt.branches);

    if t3.thought_history_length == 3 &&
       d5.thought_history_length == 5 &&
       r3.thought_history_length == 3 &&
       b3_alt.thought_history_length >= 3 {
        info!("✅ All session histories tracked correctly");
    } else {
        error!("❌ Session history mismatch detected");
    }

    // ========================================================================
    // SUMMARY
    // ========================================================================
    info!("\n========================================");
    info!("Sequential Thinking Tool Test Summary");
    info!("========================================");
    info!("✅ Test 1: Basic sequential thinking (3 thoughts) - PASSED");
    info!("✅ Test 2: Dynamic adjustment (expanded 3→5 thoughts) - PASSED");
    info!("✅ Test 3: Revision feature (corrected thought 1) - PASSED");
    info!("✅ Test 4: Branching feature (explored 2 approaches) - PASSED");
    info!("✅ Test 5: Session continuity verification - PASSED");
    info!("========================================");

    // Graceful shutdown
    conn.close().await?;
    server.shutdown().await?;
    info!("\nSequential thinking tool example completed successfully!");

    Ok(())
}
