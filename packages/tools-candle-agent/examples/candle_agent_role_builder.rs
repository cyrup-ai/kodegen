//! Candle Agent Role Builder - Complete API Demo
//!
//! Demonstrates the agent role builder pattern adapted from ARCHITECTURE.md
//! with working Candle ML framework integration.
//!
//! Run with: cargo run --example candle_agent_role_builder --release

use kodegen_candle_agent::prelude::*;
use std::io::{self, Write};
use tokio_stream::StreamExt;

/// Example 1: Full Agent Role with Advanced Configuration
/// Demonstrates role-level defaults and agent-level overrides
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("🚀 Candle Agent Role Builder - Complete API Demo");
    println!("{}", "=".repeat(80));

    // Example 1: Simple agent role with defaults
    println!("\n📝 Example 1: Simple Agent Role\n");

    let mut stream = CandleFluentAi::agent_role("rusty-assistant")
        .temperature(0.7)
        .max_tokens(2048)
        .system_prompt(
            "You are a helpful Rust programming assistant. \
             Provide clear, concise answers with code examples when appropriate.",
        )
        .on_chunk(|chunk| async move {
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent()
        .chat(|_conversation| async move {
            CandleChatLoop::UserPrompt(
                "What are the key differences between &str and String in Rust?".to_string(),
            )
        })?;

    while let Some(chunk) = stream.next().await {
        if let CandleMessageChunk::Complete {
            token_count,
            elapsed_secs,
            tokens_per_sec,
            ..
        } = chunk
        {
            println!("\n");
            if let (Some(tokens), Some(elapsed), Some(tps)) =
                (token_count, elapsed_secs, tokens_per_sec)
            {
                println!("✅ Generation complete!");
                println!("   Tokens: {}", tokens);
                println!("   Time: {:.2}s", elapsed);
                println!("   Speed: {:.2} tokens/sec", tps);
            }
        }
    }

    // Example 2: Role defaults with agent override
    println!("\n{}", "=".repeat(80));
    println!("📝 Example 2: Role Defaults + Agent Override\n");

    // Create role with default chunk handler
    let role_builder = CandleFluentAi::agent_role("helpful-assistant")
        .temperature(0.5)
        .max_tokens(1500)
        .system_prompt("You are a helpful assistant.")
        .on_chunk(|chunk| async move {
            // Role-level default: basic printing
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("[ROLE] {}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        });

    // Agent overrides the chunk handler
    let mut stream = role_builder
        .into_agent()
        .on_chunk(|chunk| async move {
            // Agent-level override: fancy printing
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("[AGENT] {}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .chat(|_conversation| async move {
            CandleChatLoop::UserPrompt("Count from 1 to 5".to_string())
        })?;

    while let Some(_chunk) = stream.next().await {
        // Chunks already printed via on_chunk handler
    }

    // Example 3: Temperature variations
    println!("\n\n{}", "=".repeat(80));
    println!("📝 Example 3: Temperature Settings\n");

    println!("Low temperature (0.3) for deterministic output:\n");
    let mut stream = CandleFluentAi::agent_role("precise-assistant")
        .temperature(0.3)
        .max_tokens(500)
        .system_prompt("Be precise and deterministic in your responses.")
        .on_chunk(|chunk| async move {
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent()
        .chat(
            |_conversation| async move { CandleChatLoop::UserPrompt("What is 7 * 8?".to_string()) },
        )?;

    while let Some(_chunk) = stream.next().await {}

    println!("\n\nHigh temperature (1.2) for creative output:\n");
    let mut stream = CandleFluentAi::agent_role("creative-assistant")
        .temperature(1.2)
        .max_tokens(500)
        .system_prompt("Be creative and imaginative in your responses.")
        .on_chunk(|chunk| async move {
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent()
        .chat(|_conversation| async move {
            CandleChatLoop::UserPrompt("Describe a sunset in three words.".to_string())
        })?;

    while let Some(_chunk) = stream.next().await {}

    // Example 4: Interactive-style chat loop
    println!("\n\n{}", "=".repeat(80));
    println!("📝 Example 4: Chat Loop Pattern\n");

    let mut stream = CandleFluentAi::agent_role("conversational-assistant")
        .temperature(0.7)
        .max_tokens(1000)
        .system_prompt("You are a friendly conversational assistant. Keep responses concise.")
        .on_chunk(|chunk| async move {
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent()
        .chat(|_conversation| async move {
            // Simple one-shot query
            CandleChatLoop::UserPrompt("Hello! What's your name?".to_string())
        })?;

    while let Some(_chunk) = stream.next().await {}

    // Example 5: System prompt variations
    println!("\n\n{}", "=".repeat(80));
    println!("📝 Example 5: System Prompt Customization\n");

    let mut stream = CandleFluentAi::agent_role("step-by-step-thinker")
        .temperature(0.6)
        .max_tokens(2000)
        .system_prompt(
            "You are an analytical assistant. For every question:\n\
             1. Break down the problem\n\
             2. Show your reasoning\n\
             3. Provide a clear answer\n\
             Use <think> tags for your internal reasoning.",
        )
        .on_chunk(|chunk| async move {
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent()
        .chat(|_conversation| async move {
            CandleChatLoop::UserPrompt(
                "How would you optimize a Rust Vec that frequently inserts at the beginning?"
                    .to_string(),
            )
        })?;

    while let Some(_chunk) = stream.next().await {}

    // Example 6: Multi-step reasoning
    println!("\n\n{}", "=".repeat(80));
    println!("📝 Example 6: Multi-Step Problem Solving\n");

    let mut stream = CandleFluentAi::agent_role("problem-solver")
        .temperature(0.5)
        .max_tokens(3000)
        .system_prompt(
            "You excel at breaking down complex problems into steps. \
             Show your work and explain each step clearly."
        )
        .on_chunk(|chunk| async move {
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent()
        .chat(|_conversation| async move {
            CandleChatLoop::UserPrompt(
                "Solve: A train travels 120 km in 2 hours. Another train travels 90 km in 1.5 hours. \
                 Which is faster and by how much?".to_string()
            )
        })?;

    while let Some(_chunk) = stream.next().await {}

    println!("\n\n{}", "=".repeat(80));
    println!("✅ All examples completed successfully!");
    println!("{}", "=".repeat(80));

    Ok(())
}
