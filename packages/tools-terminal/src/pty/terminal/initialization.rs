use std::{io::{self, BufWriter, Read, Write}, sync::{atomic::Ordering, Arc}};
use log::error;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use tokio::{sync::Mutex, task};

use super::{shell::get_default_shell, types::Terminal};

impl Terminal {
    /// Initialize the terminal with a command and spawn PTY I/O tasks
    ///
    /// # Threading Model
    ///
    /// This method spawns two background threads using `tokio::task::spawn_blocking`:
    ///
    /// 1. **Child Process Management**: Spawns the command in the PTY slave and waits for
    ///    process completion. The slave handle is dropped when the process exits.
    ///
    /// 2. **PTY Output Reading**: Reads output from the PTY master in a loop and feeds it
    ///    to the terminal parser. The `portable-pty` crate provides `Box<dyn Read + Send>`
    ///    (synchronous I/O), so we use `spawn_blocking` to avoid blocking the async executor.
    ///    Output is automatically processed by the VT100 parser to update the terminal screen.
    ///
    /// # Why `spawn_blocking`?
    ///
    /// The `portable-pty` API provides synchronous readers that implement `std::io::Read`,
    /// not `tokio::io::AsyncRead`. Using `spawn_blocking` allows us to perform blocking I/O
    /// operations without blocking the Tokio runtime's worker threads. This is the recommended
    /// approach for integrating synchronous I/O with async Rust.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if initialization succeeds, or an error if PTY setup fails.
    pub async fn init(&mut self) -> io::Result<()> {
        // GUARD: Prevent double initialization (check if tasks are already running)
        if self.writer_task.is_some() || self.reader_task.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Terminal already initialized, cannot call init() again"
            ));
        }

        // Build CommandBuilder from config
        let mut cmd = if let Some(ref command) = self.config.command {
            if self.config.shell {
                // Run command through shell
                // Use custom shell if specified, otherwise detect default
                let default_shell = get_default_shell();
                let shell_exe = self.config.shell_path
                    .as_deref()
                    .unwrap_or(&default_shell);
                let mut builder = CommandBuilder::new(shell_exe);
                builder.arg("-c");
                builder.arg(command);
                builder
            } else {
                // Parse command and args (simple split on whitespace)
                let parts: Vec<&str> = command.split_whitespace().collect();
                if parts.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Empty command provided",
                    ));
                }
                let mut builder = CommandBuilder::new(parts[0]);
                for arg in &parts[1..] {
                    builder.arg(arg);
                }
                builder
            }
        } else if self.config.shell {
            // Just run shell with no command
            let default_shell = get_default_shell();
            let shell_exe = self.config.shell_path
                .as_deref()
                .unwrap_or(&default_shell);
            CommandBuilder::new(shell_exe)
        } else {
            // Default to shell if nothing specified
            let default_shell = get_default_shell();
            let shell_exe = self.config.shell_path
                .as_deref()
                .unwrap_or(&default_shell);
            CommandBuilder::new(shell_exe)
        };

        // Apply working directory if specified
        if let Some(ref cwd) = self.config.cwd {
            cmd.cwd(cwd);
        }

        // Apply environment variables
        for (key, value) in &self.config.env_vars {
            cmd.env(key, value);
        }

        let pty_system = NativePtySystem::default();
        let pair = match pty_system.openpty(PtySize {
            rows: self.size.rows,
            cols: self.size.cols,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(pair) => pair,
            Err(e) => {
                error!("Failed to open PTY: {e}");
                return Err(io::Error::other(
                    format!("Failed to open PTY: {e}"),
                ));
            }
        };

        // Spawn child process and wrap in Arc<Mutex<>> for shared access
        let child = match pair.slave.spawn_command(cmd) {
            Ok(child) => child,
            Err(e) => {
                error!("Failed to spawn command in PTY: {e}");
                return Err(io::Error::other(
                    format!("Failed to spawn command in PTY: {e}"),
                ));
            }
        };

        let child_arc = Arc::new(Mutex::new(child));
        self.child_process = Some(child_arc.clone());

        // Spawn task to wait for child process to complete
        tokio::spawn(async move {
            // Use async lock since we're in an async context
            let mut child = child_arc.lock().await;
            if let Err(e) = child.wait() {
                error!("Failed to wait for PTY child process: {e}");
                // Continue drop(pair.slave) even on wait error
            }
            drop(child);
            drop(pair.slave);
        });

        // Create reader to get output from the terminal
        let mut reader = match pair.master.try_clone_reader() {
            // Handle this unwrap too
            Ok(reader) => reader,
            Err(e) => {
                error!("Failed to clone PTY reader: {e}");
                // CLEANUP: Kill child before returning
                if let Some(child_ref) = &self.child_process {
                    let mut child_guard = child_ref.lock().await;
                    let _ = child_guard.kill();
                }
                self.child_process = None;
                return Err(io::Error::other(e));
            }
        };

        // portable-pty provides synchronous I/O. Using spawn_blocking is the correct
        // tokio pattern for handling blocking operations without blocking the runtime.
        let parser = self.parser.clone();
        let pty_closed_flag = self.pty_closed.clone(); // Clone Arc for the task

        // Process output from the terminal
        let reader_handle = task::spawn_blocking(move || {
            let mut buf = [0u8; 65536];  // 64KB buffer for better throughput
            let mut processed_buf = Vec::with_capacity(65536);  // Pre-allocate to avoid reallocs
            loop {
                let size = match reader.read(&mut buf) {
                    Ok(size) => size,
                    Err(e) => {
                        // Check for specific errors like BrokenPipe?
                        if e.kind() == io::ErrorKind::BrokenPipe {
                            log::info!("PTY reader returned BrokenPipe.");
                        } else {
                            error!("Failed to read from PTY: {e}");
                        }
                        break; // Exit loop on read error or EOF
                    }
                };

                if size == 0 {
                    break;
                }

                if size > 0 {
                    processed_buf.extend_from_slice(&buf[..size]);
                    match parser.write() {
                        // Handle potential poison error
                        Ok(mut p) => p.process(&processed_buf),
                        Err(e) => {
                            error!(
                                "Parser lock poisoned: {e}. Stopping PTY output processing."
                            );
                            break; // Exit loop if lock is poisoned
                        }
                    }
                    processed_buf.clear();
                }
            }
            // Set the flag when the loop finishes
            pty_closed_flag.store(true, Ordering::SeqCst);
            log::info!("PTY output processing task finished.");
        });

        self.reader_task = Some(reader_handle);

        // Take writer directly from master (no mutex needed - master will be moved into writer task)
        let mut writer = match pair.master.take_writer() {
            Ok(writer) => BufWriter::new(writer),
            Err(e) => {
                error!("Failed to take PTY writer: {e}");
                // CLEANUP: Kill everything spawned so far
                self.cleanup_on_init_error().await;
                return Err(io::Error::other(e));
            }
        };

        // Move master into writer task to keep PTY file descriptors alive
        let pty_master = pair.master;

        let mut rx = match self.receiver.take() {
            // Added 'mut' here
            Some(rx) => rx,
            None => {
                error!("Terminal receiver already taken during init");
                // CLEANUP: Kill everything spawned so far
                self.cleanup_on_init_error().await;
                return Err(io::Error::other("Receiver already taken"));
            }
        };

        // Process input to the terminal
        let writer_handle = tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if let Err(e) = writer.write_all(&bytes) {
                    error!("Failed to write to PTY: {e}");
                    break;
                }
                if let Err(e) = writer.flush() {
                    error!("Failed to flush PTY writer: {e}");
                    break;
                }
            }
            // Keep the master alive until the writer task ends
            drop(pty_master);
        });

        self.writer_task = Some(writer_handle);

        Ok(())
    }

    /// Helper: Clean up resources on `init()` failure
    ///
    /// This method is called when `init()` fails after spawning resources.
    /// It asynchronously cleans up:
    /// - Child process (kills it)
    /// - Reader task (drops handle to signal cancellation)
    ///
    /// Note: PTY master is not stored in self, so it's dropped automatically when `init()` returns.
    pub(super) async fn cleanup_on_init_error(&mut self) {
        // 1. Kill child process
        if let Some(child_ref) = &self.child_process {
            let mut child_guard = child_ref.lock().await;
            if let Err(e) = child_guard.kill() {
                log::debug!("Error killing child during init cleanup: {e}");
            }
        }
        self.child_process = None;

        // 2. Cancel reader task (drop handle to signal cancellation)
        //    Reader will detect closed PTY and exit naturally
        self.reader_task = None;

        log::info!("Cleaned up resources after init() failure");
    }
}
