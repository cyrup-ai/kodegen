//! Candle Fluent Builder Example
//!
//! Demonstrates the complete fluent API chain:
//! CandleFluentAi::agent_role() -> builder methods -> .into_agent() -> .chat()
//!
//! # Usage
//!
//! ```bash
//! # Uses qwen-3 by default (no .model() needed!)
//! cargo run --example fluent_builder --release
//!
//! # With custom query
//! cargo run --example fluent_builder --release -- --query "Explain quantum computing"
//! ```

use clap::Parser;
use kodegen_candle_agent::prelude::*;
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(author, version, about = "Candle Fluent Builder Example")]
struct Args {
    /// User query to process
    #[arg(
        long,
        default_value = "Solve this step by step: If x + 5 = 12, what is x?"
    )]
    query: String,

    /// Temperature (0.0-2.0)
    #[arg(long, default_value_t = 0.7)]
    temperature: f64,

    /// Maximum tokens to generate
    #[arg(long, default_value_t = 2048)]
    max_tokens: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = Args::parse();

    println!("🚀 Candle Fluent Builder Example");
    println!("{}", "=".repeat(80));
    println!("Query: {}", args.query);
    println!("Temperature: {}", args.temperature);
    println!("Max Tokens: {}", args.max_tokens);
    println!("Model: qwen-3 (default)");
    println!("{}\n", "=".repeat(80));

    // Example 1: Simple one-shot query with fluent chain
    println!("📝 Example 1: Simple One-Shot Query\n");

    let query = args.query.clone();
    let mut stream = CandleFluentAi::agent_role("helpful-assistant")
        .temperature(args.temperature)
        .max_tokens(args.max_tokens)
        .system_prompt("You are a helpful AI assistant. Think step-by-step and be concise.")
        .on_chunk(|chunk| async move {
            // Stream each token to stdout in real-time
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent()
        .chat(move |_conversation| {
            let query = query.clone();
            async move { CandleChatLoop::UserPrompt(query) }
        })?;

    // Consume the stream
    use tokio_stream::StreamExt;
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

    // Example 2: Interactive chat loop
    println!("\n{}", "=".repeat(80));
    println!("📝 Example 2: Interactive Chat Loop\n");

    let mut stream = CandleFluentAi::agent_role("math-tutor")
        .temperature(0.3) // Lower temperature for more focused responses
        .max_tokens(1024)
        .system_prompt("You are a patient math tutor. Explain concepts clearly and show your work.")
        .on_chunk(|chunk| async move {
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent()
        .chat(|_conversation| async move {
            // In a real app, you could inspect conversation history here
            CandleChatLoop::UserPrompt("What is 15 * 24?".to_string())
        })?;

    while let Some(_chunk) = stream.next().await {
        // Chunks already printed via on_chunk handler
    }

    // Example 3: Reasoning with thinking
    println!("\n\n{}", "=".repeat(80));
    println!("📝 Example 3: Deep Reasoning\n");

    let mut stream = CandleFluentAi::agent_role("reasoner")
        .temperature(0.7)
        .max_tokens(3000)
        .system_prompt(
            "You are an expert at analytical reasoning. \
             Break down complex problems systematically using the <think> tag for your reasoning process."
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
                "What are the trade-offs between monolithic and microservices architectures?".to_string()
            )
        })?;

    while let Some(_chunk) = stream.next().await {
        // Chunks already printed via on_chunk handler
    }

    println!("\n\n{}", "=".repeat(80));
    println!("✅ All examples completed!");
    println!("{}", "=".repeat(80));

    Ok(())
}
