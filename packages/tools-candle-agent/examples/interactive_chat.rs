//! Interactive Chat Example
//!
//! Demonstrates how to build an interactive CLI tool with stdin prompting.
//!
//! This example shows:
//! - Reading user input from stdin in the chat handler
//! - Handling exit commands gracefully
//! - Real-time output streaming with on_chunk
//! - Proper stream consumption for multi-turn conversations
//!
//! # Usage
//!
//! ```bash
//! cargo run --example interactive_chat --release
//! ```

use clap::Parser;
use kodegen_candle_agent::prelude::*;
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(author, version, about = "Interactive Chat Example")]
struct Args {
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

    println!("🚀 Interactive Chat Example");
    println!("{}", "=".repeat(60));
    println!("Type your message (or 'exit' to quit)");
    println!("Commands: /exit, /quit, exit, quit");
    println!("{}\n", "=".repeat(60));

    // Build agent with fluent API
    let agent = CandleFluentAi::agent_role("helpful-assistant")
        .temperature(args.temperature)
        .max_tokens(args.max_tokens)
        .system_prompt("You are a helpful AI assistant. Be concise and friendly.")
        .on_chunk(|chunk| async move {
            // Stream tokens in real-time
            if let CandleMessageChunk::Text(ref text) = chunk {
                print!("{}", text);
                io::stdout().flush().unwrap();
            }
            chunk
        })
        .into_agent();

    // Start interactive chat loop
    let stream = agent.chat(|_conversation| async move {
        print!("You: ");
        io::stdout().flush().unwrap();

        // Use tokio async stdin
        use tokio::io::{AsyncBufReadExt, BufReader};
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut input = String::new();

        match reader.read_line(&mut input).await {
            Ok(_) => {
                let input = input.trim();

                // Handle exit commands
                if input.eq_ignore_ascii_case("exit")
                    || input.eq_ignore_ascii_case("quit")
                    || input.eq_ignore_ascii_case("/exit")
                    || input.eq_ignore_ascii_case("/quit")
                {
                    println!("Goodbye! 👋");
                    return CandleChatLoop::Break;
                }

                // Handle empty input
                if input.is_empty() {
                    return CandleChatLoop::Reprompt(String::new());
                }

                // Send to model
                CandleChatLoop::UserPrompt(input.to_string())
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                CandleChatLoop::Break
            }
        }
    })?;

    use tokio_stream::StreamExt;
    tokio::pin!(stream);

    // Consume stream - displays all conversation turns
    print!("Assistant: ");
    while let Some(chunk) = stream.next().await {
        match chunk {
            CandleMessageChunk::Complete {
                token_count,
                elapsed_secs,
                tokens_per_sec,
                ..
            } => {
                println!("\n");

                // Display generation statistics
                if let (Some(tc), Some(es), Some(tps)) = (token_count, elapsed_secs, tokens_per_sec)
                {
                    println!("✓ {} tokens in {:.2}s ({:.1} tokens/sec)", tc, es, tps);
                }

                println!(); // Blank line before next prompt

                // Stream continues - loop will call handler again for next input
                print!("Assistant: ");
            }
            CandleMessageChunk::Error(err) => {
                eprintln!("\n❌ Error: {}", err);
                println!();
            }
            CandleMessageChunk::ToolCallStart { name, .. } => {
                println!("\n🔧 [Calling tool: {}]", name);
            }
            _ => {
                // Text chunks already printed via on_chunk handler
            }
        }
    }

    Ok(())
}
