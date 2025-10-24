//! Subprocess transport implementation using Claude Code CLI

use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::Transport;
use crate::error::{ClaudeError, Result};
use crate::types::options::ClaudeAgentOptions;

use super::config::{DEFAULT_MAX_BUFFER_SIZE, PromptInput};

/// Subprocess transport for Claude Code CLI
pub struct SubprocessTransport {
    pub(super) prompt: PromptInput,
    pub(super) options: ClaudeAgentOptions,
    pub(super) cli_path: PathBuf,
    pub(super) cwd: Option<PathBuf>,
    pub(super) process: Option<Child>,
    pub(super) stdin: Option<ChildStdin>,
    pub(super) stdout: Option<BufReader<ChildStdout>>,
    pub(super) ready: Arc<AtomicBool>,
    pub(super) max_buffer_size: usize,
    pub(super) reader_task: Option<JoinHandle<()>>,
    pub(super) stderr_task: Option<JoinHandle<()>>,
}

impl SubprocessTransport {
    /// Create a new subprocess transport
    ///
    /// # Arguments
    /// * `prompt` - The prompt input (string or stream)
    /// * `options` - Configuration options
    /// * `cli_path` - Optional path to Claude Code CLI (will search if None)
    ///
    /// # Errors
    /// Returns error if CLI cannot be found
    pub fn new(
        prompt: PromptInput,
        options: ClaudeAgentOptions,
        cli_path: Option<PathBuf>,
    ) -> Result<Self> {
        let cli_path = if let Some(path) = cli_path {
            path
        } else {
            Self::find_cli()?
        };

        let cwd = options.cwd.clone();
        let max_buffer_size = options.max_buffer_size.unwrap_or(DEFAULT_MAX_BUFFER_SIZE);

        Ok(Self {
            prompt,
            options,
            cli_path,
            cwd,
            process: None,
            stdin: None,
            stdout: None,
            ready: Arc::new(AtomicBool::new(false)),
            max_buffer_size,
            reader_task: None,
            stderr_task: None,
        })
    }

    /// Find Claude Code CLI binary
    ///
    /// # Errors
    /// Returns error if CLI cannot be found in PATH or common locations
    pub fn find_cli() -> Result<PathBuf> {
        // Try using 'which' crate first
        if let Ok(path) = which::which("claude") {
            return Ok(path);
        }

        // Manual search in common locations
        let home = env::var("HOME").unwrap_or_else(|_| String::from("/root"));
        let locations = vec![
            PathBuf::from(home.clone()).join(".npm-global/bin/claude"),
            PathBuf::from("/usr/local/bin/claude"),
            PathBuf::from(home.clone()).join(".local/bin/claude"),
            PathBuf::from(home.clone()).join("node_modules/.bin/claude"),
            PathBuf::from(home).join(".yarn/bin/claude"),
        ];

        for path in locations {
            if path.exists() && path.is_file() {
                return Ok(path);
            }
        }

        Err(ClaudeError::cli_not_found())
    }
}

impl Transport for SubprocessTransport {
    async fn connect(&mut self) -> Result<()> {
        self.connect_impl().await
    }

    async fn write(&mut self, data: &str) -> Result<()> {
        if !self.is_ready() {
            return Err(ClaudeError::transport("Transport is not ready for writing"));
        }

        let stdin = self
            .stdin
            .as_mut()
            .ok_or_else(|| ClaudeError::transport("stdin not available"))?;

        stdin
            .write_all(data.as_bytes())
            .await
            .map_err(|e| ClaudeError::transport(format!("Failed to write to stdin: {e}")))?;

        stdin
            .flush()
            .await
            .map_err(|e| ClaudeError::transport(format!("Failed to flush stdin: {e}")))?;

        Ok(())
    }

    async fn end_input(&mut self) -> Result<()> {
        if let Some(mut stdin) = self.stdin.take() {
            stdin
                .shutdown()
                .await
                .map_err(|e| ClaudeError::transport(format!("Failed to close stdin: {e}")))?;
        }
        Ok(())
    }

    fn read_messages(&mut self) -> mpsc::UnboundedReceiver<Result<serde_json::Value>> {
        self.read_messages_impl()
    }

    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    async fn close(&mut self) -> Result<()> {
        self.close_impl().await
    }
}

impl Drop for SubprocessTransport {
    fn drop(&mut self) {
        self.drop_impl();
    }
}
