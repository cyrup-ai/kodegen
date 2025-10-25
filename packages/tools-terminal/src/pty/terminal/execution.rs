use std::io;
use vt100::{Parser, Screen};

use super::types::Terminal;

impl Terminal {
    /// Execute a command in the terminal and return the resulting screen
    ///
    /// Returns a `JoinHandle` that resolves to the final screen state when the command completes.
    ///
    /// # Errors
    /// Returns error if:
    /// - Terminal already initialized (call `exec()` only once per Terminal)
    /// - PTY creation fails
    /// - Command spawn fails
    /// - System resource limits reached
    ///
    /// # Example
    /// ```no_run
    /// # use std::error::Error;
    /// # async fn example() -> Result<(), Box<dyn Error>> {
    /// let mut term = Terminal::builder().build();
    /// let handle = term.exec("ls -la")?;
    /// let screen = handle.await?;
    /// println!("{}", screen.contents());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exec(
        &mut self,
        command: impl Into<String> + Send + 'static,
    ) -> io::Result<tokio::task::JoinHandle<Screen>> {
        let command_str = command.into();

        // GUARD: Prevent double initialization (check if tasks are already running)
        if self.writer_task.is_some() || self.reader_task.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Terminal already initialized, cannot call exec() again",
            ));
        }

        // Update config to run this command through shell
        self.config.command = Some(command_str);
        self.config.shell = true;

        // Initialize the terminal with the configured command
        self.init().await?;

        // Clone self to move into the async task
        let terminal = self.clone();

        // Spawn an async task that waits for the command to complete
        Ok(tokio::spawn(async move {
            // Poll until the PTY is closed (command finished)
            while !terminal.is_pty_closed() {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            // Try to get the screen, retrying a few times if the lock is busy
            for attempt in 0..10 {
                if let Some(screen) = terminal.screen() {
                    return screen;
                }
                if attempt < 9 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }

            // If we still can't get the screen after retries, return a blank screen
            // This should be extremely rare (only if the parser lock is poisoned)
            log::error!("Failed to acquire parser screen after retries, returning empty screen");
            Parser::new(terminal.size.rows, terminal.size.cols, 0)
                .screen()
                .clone()
        }))
    }
}
