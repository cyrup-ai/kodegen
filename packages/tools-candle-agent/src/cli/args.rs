//! CLI argument parsing and validation
//!
//! This module defines the CLI arguments structure for the interactive chat application.

use std::path::PathBuf;

/// CLI arguments for the chat application
#[derive(Debug, Clone)]
pub struct CliArgs {
    /// Model to use for inference (optional - will prompt if not provided)
    pub model: Option<String>,

    /// Agent role name (defaults to "CYRUP.ai")
    pub agent_role: String,

    /// System prompt to configure assistant behavior
    pub system_prompt: Option<String>,

    /// Documents to load as context (files, directories, URLs)
    pub documents: Vec<PathBuf>,

    /// Temperature for sampling (0.0-2.0, defaults to 0.0)
    pub temperature: f64,

    /// Maximum tokens to generate (optional - uses model's capability if not set)
    pub max_tokens: Option<u64>,

    /// Memory read timeout in milliseconds (defaults to 5000ms)
    pub memory_read_timeout: u64,

    /// Tool/plugin WASM paths to load (repeatable)
    pub tools: Vec<String>,

    /// Embedding model for memory system (optional - uses builder's default if not provided)
    /// Supported registry keys: dunzhang/stella_en_1.5B_v5, bert, gte-qwen, jina-bert, nvembed, clip-vision
    pub embedding_model: Option<String>,

    /// Interactive mode (default: true)
    pub interactive: bool,

    /// Initial user message (non-interactive mode)
    pub message: Option<String>,

    /// Config file path
    pub config: Option<PathBuf>,

    /// Verbose logging
    pub verbose: bool,
}

impl Default for CliArgs {
    fn default() -> Self {
        Self {
            model: None,
            agent_role: "CYRUP.ai".to_string(),
            system_prompt: None,
            documents: Vec::new(),
            temperature: 0.0,          // Task specifies 0.0, not 0.7
            max_tokens: None,          // Let model decide based on TextToTextCapability
            memory_read_timeout: 5000, // 5 seconds
            tools: Vec::new(),
            embedding_model: None, // Uses EmbeddingConfig::default() if not provided
            interactive: true,
            message: None,
            config: None,
            verbose: false,
        }
    }
}

impl CliArgs {
    /// Create new CLI args with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse from environment args
    pub fn from_env() -> Self {
        let args: Vec<String> = std::env::args().collect();
        Self::from_args(&args)
    }

    /// Parse from argument vector
    pub fn from_args(args: &[String]) -> Self {
        let mut cli_args = Self::default();
        let mut i = 1; // Skip program name

        while i < args.len() {
            match args[i].as_str() {
                "-m" | "--model" => {
                    i += 1;
                    if i < args.len() {
                        cli_args.model = Some(args[i].clone());
                    }
                }
                "-r" | "--role" => {
                    i += 1;
                    if i < args.len() {
                        cli_args.agent_role = args[i].clone();
                    }
                }
                "-s" | "--system-prompt" => {
                    i += 1;
                    if i < args.len() {
                        cli_args.system_prompt = Some(args[i].clone());
                    }
                }
                "-d" | "--document" => {
                    i += 1;
                    if i < args.len() {
                        cli_args.documents.push(PathBuf::from(&args[i]));
                    }
                }
                "-t" | "--temperature" => {
                    i += 1;
                    if i < args.len()
                        && let Ok(temp) = args[i].parse::<f64>()
                    {
                        cli_args.temperature = temp;
                    }
                }
                "--max-tokens" => {
                    i += 1;
                    if i < args.len()
                        && let Ok(tokens) = args[i].parse::<u64>()
                    {
                        cli_args.max_tokens = Some(tokens);
                    }
                }
                "--memory-read-timeout" => {
                    i += 1;
                    if i < args.len()
                        && let Ok(timeout) = args[i].parse::<u64>()
                    {
                        cli_args.memory_read_timeout = timeout;
                    }
                }
                "--tool" => {
                    i += 1;
                    if i < args.len() {
                        cli_args.tools.push(args[i].clone());
                    }
                }
                "--embedding-model" => {
                    i += 1;
                    if i < args.len() {
                        cli_args.embedding_model = Some(args[i].clone());
                    }
                }
                "--message" => {
                    i += 1;
                    if i < args.len() {
                        cli_args.message = Some(args[i].clone());
                        cli_args.interactive = false;
                    }
                }
                "--config" => {
                    i += 1;
                    if i < args.len() {
                        cli_args.config = Some(PathBuf::from(&args[i]));
                    }
                }
                "-v" | "--verbose" => {
                    cli_args.verbose = true;
                }
                "--no-interactive" => {
                    cli_args.interactive = false;
                }
                _ => {
                    // Treat unknown args as documents
                    if !args[i].starts_with('-') {
                        cli_args.documents.push(PathBuf::from(&args[i]));
                    }
                }
            }
            i += 1;
        }

        cli_args
    }

    /// Validate arguments
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(format!(
                "Temperature must be between 0.0 and 2.0, got {}",
                self.temperature
            ));
        }

        if let Some(tokens) = self.max_tokens
            && tokens == 0
        {
            return Err("Max tokens must be greater than 0".to_string());
        }

        if self.memory_read_timeout == 0 {
            return Err("Memory read timeout must be greater than 0".to_string());
        }

        Ok(())
    }
}
