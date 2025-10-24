//! Lifecycle management for subprocess transport (connect, close)

use std::collections::HashMap;
use std::env;
use std::process::Stdio;
use std::sync::atomic::Ordering;

use crate::VERSION;
use crate::error::{ClaudeError, Result};

use super::command::CommandBuilder;
use super::config::{DANGEROUS_ENV_VARS, PromptInput};
use super::transport::SubprocessTransport;

impl SubprocessTransport {
    /// Connect to the subprocess transport
    ///
    /// This method spawns the Claude Code CLI process and sets up stdio pipes.
    ///
    /// # Errors
    /// Returns error if process spawning fails or stdio handles cannot be obtained
    pub(super) async fn connect_impl(&mut self) -> Result<()> {
        if self.process.is_some() {
            return Ok(());
        }

        let builder = CommandBuilder::new(&self.cli_path, &self.prompt, &self.options);
        let mut cmd = builder.build();

        // Set up environment - filter dangerous variables
        let mut process_env = env::vars().collect::<HashMap<_, _>>();

        // Only add user-provided env vars that are not in the dangerous list
        for (key, value) in &self.options.env {
            if !DANGEROUS_ENV_VARS.contains(&key.as_str()) {
                process_env.insert(key.clone(), value.clone());
            }
        }

        process_env.insert("CLAUDE_CODE_ENTRYPOINT".to_string(), "sdk-rust".to_string());
        process_env.insert("CLAUDE_AGENT_SDK_VERSION".to_string(), VERSION.to_string());

        if let Some(ref cwd) = self.cwd {
            process_env.insert("PWD".to_string(), cwd.to_string_lossy().to_string());
            cmd.current_dir(cwd);
        }

        cmd.envs(process_env);

        // Set up stdio
        // IMPORTANT: We pipe stderr instead of inheriting to prevent the child process
        // from manipulating the parent terminal state. Inheriting stderr gives the child
        // access to the terminal, which can leave it in a corrupted state.
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()); // Pipe stderr to prevent terminal manipulation

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| {
            if let Some(ref cwd) = self.cwd
                && !cwd.exists()
            {
                #[cfg(debug_assertions)]
                return ClaudeError::connection(format!(
                    "Working directory does not exist: {}",
                    cwd.display()
                ));
                #[cfg(not(debug_assertions))]
                return ClaudeError::connection("Working directory does not exist".to_string());
            }
            ClaudeError::connection(format!("Failed to start Claude Code: {e}"))
        })?;

        // Get stdin, stdout, and stderr
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ClaudeError::connection("Failed to get stdin handle"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ClaudeError::connection("Failed to get stdout handle"))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ClaudeError::connection("Failed to get stderr handle"))?;

        // Spawn task to consume stderr to prevent blocking
        // We forward it to parent stderr for visibility
        let stderr_task = tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut stderr = stderr;
            let mut buffer = vec![0u8; 4096];

            loop {
                match stderr.read(&mut buffer).await {
                    Ok(0) | Err(_) => break, // EOF
                    Ok(n) => {
                        // Forward stderr to parent's stderr
                        let _ = std::io::Write::write_all(&mut std::io::stderr(), &buffer[..n]);
                    }
                }
            }
        });

        // Store handles
        self.stdin = Some(stdin);
        self.stdout = Some(tokio::io::BufReader::new(stdout));
        self.process = Some(child);
        self.stderr_task = Some(stderr_task);
        self.ready.store(true, Ordering::SeqCst);

        // For string mode, close stdin immediately
        if matches!(self.prompt, PromptInput::String(_))
            && let Some(mut stdin) = self.stdin.take()
        {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.shutdown().await;
        }

        Ok(())
    }

    /// Close the transport and clean up resources
    ///
    /// # Errors
    /// Returns error if cleanup fails
    pub(super) async fn close_impl(&mut self) -> Result<()> {
        self.ready.store(false, Ordering::SeqCst);

        // Close stdin to signal the process to exit gracefully
        if let Some(mut stdin) = self.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.shutdown().await;
        }

        // Abort reader and stderr tasks first to prevent race conditions
        if let Some(task) = self.reader_task.take() {
            task.abort();
            // Give the task a moment to clean up
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }

        self.stdout = None;

        // Try to wait for the process to exit gracefully first
        if let Some(mut child) = self.process.take() {
            // Give the process a configurable timeout to exit gracefully
            let timeout_duration = std::time::Duration::from_secs(5);

            match tokio::time::timeout(timeout_duration, child.wait()).await {
                Ok(Ok(_status)) => {
                    // Process exited gracefully
                }
                Ok(Err(e)) => {
                    return Err(ClaudeError::Io(e));
                }
                Err(_) => {
                    // Timeout - kill the process
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
        }

        Ok(())
    }

    /// Handle Drop cleanup
    pub(super) fn drop_impl(&mut self) {
        // Close stdin if still open to signal graceful shutdown
        if let Some(stdin) = self.stdin.take() {
            // Drop will close it
            drop(stdin);
        }

        // Abort reader task if running
        if let Some(task) = self.reader_task.take() {
            task.abort();
        }

        // Abort stderr task if running
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }

        // Try graceful shutdown first, then kill if needed
        if let Some(mut child) = self.process.take() {
            // Try to kill gracefully (SIGTERM on Unix)
            let _ = child.start_kill();
        }
    }
}
