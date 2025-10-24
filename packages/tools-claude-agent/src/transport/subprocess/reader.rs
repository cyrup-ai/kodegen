//! Message reading logic for subprocess transport

use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{Mutex, mpsc};

use crate::error::{ClaudeError, Result};

use super::transport::SubprocessTransport;

impl SubprocessTransport {
    /// Read messages from the subprocess output
    ///
    /// This method spawns a background task to read JSON messages from stdout.
    ///
    /// # Returns
    /// A receiver that yields parsed JSON values or errors
    pub(super) fn read_messages_impl(
        &mut self,
    ) -> mpsc::UnboundedReceiver<Result<serde_json::Value>> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Take ownership of stdout and process
        let stdout = self.stdout.take();
        let process = Arc::new(Mutex::new(self.process.take()));
        let max_buffer_size = self.max_buffer_size;

        // Spawn background task to read messages
        let task = tokio::spawn(async move {
            let Some(mut stdout) = stdout else {
                let _ = tx.send(Err(ClaudeError::connection(
                    "Not connected - stdout not available",
                )));
                return;
            };
            let mut json_buffer = String::new();

            loop {
                let mut line = String::new();

                // Add timeout to read_line to prevent hanging
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    stdout.read_line(&mut line),
                )
                .await
                {
                    Ok(Ok(0)) => break, // EOF
                    Ok(Ok(_)) => {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        // Accumulate partial JSON until we can parse it
                        json_buffer.push_str(line);

                        if json_buffer.len() > max_buffer_size {
                            let _ = tx.send(Err(ClaudeError::JsonDecode(
                                serde_json::Error::io(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!(
                                        "JSON message exceeded maximum buffer size of {max_buffer_size} bytes"
                                    ),
                                )),
                            )));
                            json_buffer.clear();
                            continue;
                        }

                        // Try to parse JSON
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&json_buffer) {
                            json_buffer.clear();
                            if tx.send(Ok(data)).is_err() {
                                // Receiver dropped, stop reading
                                break;
                            }
                        }
                        // Otherwise not complete yet, continue accumulating
                        // The timeout on read_line will handle incomplete JSON timeouts
                    }
                    Ok(Err(e)) => {
                        let _ = tx.send(Err(ClaudeError::Io(e)));
                        break;
                    }
                    Err(_) => {
                        let _ = tx.send(Err(ClaudeError::timeout("Read operation timed out")));
                        break;
                    }
                }
            }

            // Check process exit code
            if let Ok(mut process_guard) = process.try_lock()
                && let Some(mut child) = process_guard.take()
            {
                match child.wait().await {
                    Ok(status) => {
                        if !status.success()
                            && let Some(code) = status.code()
                        {
                            let _ = tx.send(Err(ClaudeError::process(
                                "Command failed",
                                code,
                                Some("Check stderr output for details".to_string()),
                            )));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(ClaudeError::Io(e)));
                    }
                }
            }
        });

        // Store task handle for cleanup
        self.reader_task = Some(task);

        rx
    }
}
