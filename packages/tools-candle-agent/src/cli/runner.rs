//! CLI runner - main orchestration for interactive chat application using fluent API
//!
//! This module demonstrates the canonical use of CandleFluentAi::agent_role() builder pattern
//! with proper defaults, smart input resolution, and all CLI flags wired to builder methods.

use anyhow::{Context, Result};
use std::io::Write;

use super::args::CliArgs;
use super::config::CliConfig;
use super::handler::{CommandResult, InputHandler, InputHandlerResult};

use crate::builders::agent_role::{CandleAgentBuilder, CandleAgentRoleBuilder, CandleFluentAi};
use crate::domain::chat::CandleChatLoop;
use crate::util::input_resolver::resolve_input;

/// CLI runner for interactive chat
pub struct CliRunner {
    args: CliArgs,
    config: CliConfig,
    handler: InputHandler,
}

impl CliRunner {
    /// Create new CLI runner from arguments
    pub fn new(args: CliArgs) -> Result<Self> {
        // Validate arguments
        args.validate()
            .map_err(|e| anyhow::anyhow!("Invalid arguments: {}", e))?;

        // Load or create config
        let config = if let Some(config_path) = &args.config {
            CliConfig::load(Some(config_path))
                .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?
        } else {
            CliConfig::load(None)
                .map_err(|e| anyhow::anyhow!("Failed to load default config: {}", e))?
        };

        let handler = InputHandler::new(config.clone());

        Ok(Self {
            args,
            config,
            handler,
        })
    }

    /// Run the CLI application using fluent API
    pub async fn run(&mut self) -> Result<()> {
        use tokio_stream::StreamExt;

        // Initialize pool maintenance thread (lazy init)
        crate::capability::registry::pool::init_maintenance();

        // Setup Ctrl+C handler for IMMEDIATE exit
        ctrlc::set_handler(move || {
            eprintln!("\n\nExiting...");
            std::process::exit(0);
        })
        .map_err(|e| anyhow::anyhow!("Failed to set Ctrl-C handler: {}", e))?;

        // Clean, minimal banner
        println!("\n╭─────────────────────────────────────╮");
        println!("│  🤖  Interactive AI Chat           │");
        println!("╰─────────────────────────────────────╯");
        println!("\nType /help for commands • Ctrl+C to exit\n");

        // Resolve system prompt using smart input resolution
        let system_prompt = if let Some(ref prompt_input) = self.args.system_prompt {
            resolve_input(prompt_input)
                .await
                .context("Failed to resolve system prompt")?
        } else {
            // Use default architect prompt from task specification
            String::from(
                r#"# Well-Informed Software Architect

You think out loud as you work through problems, sharing your process in addition to the solutions.
You track every task you do or needs doing in `TODO.md`, updating it religiously before and after a meaningful change to code.
You maintain `ARCHITECTURE.md` and carefully curate the vision for the modules we create.
You prototype exploratory code ideas, quickly putting together a prototype, so we talk about the "heart of the matter" and get on the same page.
If you don't know the answer, you ALWAYS RESEARCH on the web and talk it through with me. You know that planned takes less time in the end that hastily forged.You never pretend to have answers unless you are highly confident.
You really LOVE programming and the art of it. You craft applications that are fast, efficient, and blazing fast.
You produce clean, maintainable, *production quality* code all the time.
You are a master at debugging and fixing bugs.
You are a master at refactoring code, remembering to check for code that ALREADY EXISTS before writing new code that might duplicate existing functionality."#,
            )
        };

        // Use async closure with direct tokio stdin reading (prepare handler first)
        let handler = std::sync::Arc::new(std::sync::Mutex::new(self.handler.clone()));

        // Build agent and compute stream directly in each branch to avoid opaque type mismatch
        let stream = if let Some(registry_key) = &self.args.model {
            use crate::capability::registry::{self, TextToTextModel};

            let text_model = registry::get::<TextToTextModel>(registry_key)
                .ok_or_else(|| anyhow::anyhow!("Model not found in registry: {}", registry_key))?;

            CandleFluentAi::agent_role(&self.args.agent_role)
                .into_agent()
                .model(text_model)
                .temperature(self.args.temperature)
                .system_prompt(system_prompt.clone())
                .memory_read_timeout(self.args.memory_read_timeout)
                .max_tokens(self.args.max_tokens.unwrap_or(2000))
                .on_chunk(|chunk| async move {
                    use crate::domain::chat::message::CandleMessageChunk;
                    if let CandleMessageChunk::Text(ref text) = chunk {
                        print!("{}", text);
                        let _ = std::io::stdout().flush();
                    }
                    chunk
                })
                .chat(move |_conversation| {
                    let handler = handler.clone();
                    async move {
                        use tokio::io::{AsyncBufReadExt, BufReader};

                        print!("\n> You: ");
                        let _ = std::io::stdout().flush();

                        let stdin = tokio::io::stdin();
                        let mut reader = BufReader::new(stdin);
                        let mut input = String::new();

                        match reader.read_line(&mut input).await {
                            Ok(0) => CandleChatLoop::Break, // EOF
                            Ok(_) => {
                                let input = input.trim();

                                // Handle input via InputHandler
                                let handler_result = match handler.lock() {
                                    Ok(mut h) => h.handle(input),
                                    Err(_) => InputHandlerResult::Exit,
                                };

                                match handler_result {
                                    InputHandlerResult::Exit => {
                                        println!("Goodbye!");
                                        CandleChatLoop::Break
                                    }
                                    InputHandlerResult::Command(cmd_result) => {
                                        let output = Self::format_command_result(&cmd_result);
                                        println!("{}", output);
                                        CandleChatLoop::Reprompt(String::new())
                                    }
                                    InputHandlerResult::None => {
                                        CandleChatLoop::Reprompt(String::new())
                                    }
                                    InputHandlerResult::Chat(message) => {
                                        CandleChatLoop::UserPrompt(message)
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Input error: {}", e);
                                CandleChatLoop::Break
                            }
                        }
                    }
                })?
        } else {
            CandleFluentAi::agent_role(&self.args.agent_role)
                .into_agent()
                .temperature(self.args.temperature)
                .system_prompt(system_prompt.clone())
                .memory_read_timeout(self.args.memory_read_timeout)
                .max_tokens(self.args.max_tokens.unwrap_or(2000))
                .on_chunk(|chunk| async move {
                    use crate::domain::chat::message::CandleMessageChunk;
                    if let CandleMessageChunk::Text(ref text) = chunk {
                        print!("{}", text);
                        let _ = std::io::stdout().flush();
                    }
                    chunk
                })
                .chat(move |_conversation| {
                    let handler = handler.clone();
                    async move {
                        use tokio::io::{AsyncBufReadExt, BufReader};

                        print!("\n> You: ");
                        let _ = std::io::stdout().flush();

                        let stdin = tokio::io::stdin();
                        let mut reader = BufReader::new(stdin);
                        let mut input = String::new();

                        match reader.read_line(&mut input).await {
                            Ok(0) => CandleChatLoop::Break, // EOF
                            Ok(_) => {
                                let input = input.trim();

                                // Handle input via InputHandler
                                let handler_result = match handler.lock() {
                                    Ok(mut h) => h.handle(input),
                                    Err(_) => InputHandlerResult::Exit,
                                };

                                match handler_result {
                                    InputHandlerResult::Exit => {
                                        println!("Goodbye!");
                                        CandleChatLoop::Break
                                    }
                                    InputHandlerResult::Command(cmd_result) => {
                                        let output = Self::format_command_result(&cmd_result);
                                        println!("{}", output);
                                        CandleChatLoop::Reprompt(String::new())
                                    }
                                    InputHandlerResult::None => {
                                        CandleChatLoop::Reprompt(String::new())
                                    }
                                    InputHandlerResult::Chat(message) => {
                                        CandleChatLoop::UserPrompt(message)
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Input error: {}", e);
                                CandleChatLoop::Break
                            }
                        }
                    }
                })?
        };
        tokio::pin!(stream);

        // Consume stream
        println!("\n💭 ");
        while let Some(chunk) = stream.next().await {
            use crate::domain::chat::message::CandleMessageChunk;
            match chunk {
                CandleMessageChunk::Text(_) => {
                    // Text already printed via on_chunk handler
                }
                CandleMessageChunk::Complete { text, .. } => {
                    if !text.is_empty() {
                        print!("{}", text);
                    }
                    println!("\n");
                }
                CandleMessageChunk::Error(err) => {
                    eprintln!("\n❌ {}", err);
                }
                CandleMessageChunk::ToolCallStart { name, .. } => {
                    println!("\n🔧 {}", name);
                }
                CandleMessageChunk::ToolCallComplete { .. } => {}
                _ => {}
            }
        }

        // Save config on exit
        self.save_config()?;

        Ok(())
    }

    /// Format command result for display
    fn format_command_result(result: &CommandResult) -> String {
        match result {
            CommandResult::Help(text) => text.clone(),
            CommandResult::SavedToFile(path) => format!("Saved to: {}", path),
            CommandResult::SearchResults(results) => {
                if results.is_empty() {
                    "No results found".to_string()
                } else {
                    format!("Found {} results:\n{}", results.len(), results.join("\n"))
                }
            }
            CommandResult::ConfigChanged(msg) => msg.clone(),
            CommandResult::HistoryCleared => "History cleared".to_string(),
            CommandResult::Error(err) => format!("Error: {}", err),
        }
    }

    /// Save config to disk
    fn save_config(&self) -> Result<()> {
        self.config
            .save(self.args.config.as_deref())
            .map_err(|e| anyhow::anyhow!("Failed to save config: {}", e))
    }
}

impl Default for CliRunner {
    fn default() -> Self {
        let config = CliConfig::default();
        let handler = InputHandler::new(config.clone());

        Self {
            args: CliArgs::default(),
            config,
            handler,
        }
    }
}
