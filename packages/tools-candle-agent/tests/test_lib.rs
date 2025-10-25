// Tests extracted from src/lib.rs

use kodegen_candle_agent::prelude::*;

#[test]
fn test_architecture_md_syntax_works() -> Result<(), Box<dyn std::error::Error>> {
    // Test that ARCHITECTURE.md builder pattern still works after all fixes
    let _agent = CandleFluentAi::agent_role("test-agent")
        .temperature(0.0) // Greedy sampling example - deterministic output
        .max_tokens(1000)
        .system_prompt("You are a helpful assistant")
        .into_agent()?;

    // If this compiles, the ARCHITECTURE.md syntax is working! ✅
    Ok(())
}
