use crate::pty::Terminal;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use kodegen_mcp_tool::error::McpError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

// Constants

// Cleanup retention configuration
const CLEANUP_INTERVAL_SECS: u64 = 60; // Check every minute
const ACTIVE_SESSION_RETENTION_SECS: u64 = 5 * 60; // 5 minutes for active sessions
const COMPLETED_SESSION_RETENTION_SECS: u64 = 30; // 30 seconds for completed sessions

// Output buffer limits
const MAX_OUTPUT_BUFFER_LINES: usize = 10_000; // Maximum lines per session

// Session limits
const MAX_SESSIONS: usize = 100; // Maximum concurrent sessions

// REPL prompt patterns for detecting when a REPL is ready for input
const REPL_PROMPTS: &[&str] = &[
    ">>> ",       // Python
    "... ",       // Python continuation
    ">> ",        // R
    "> ",         // R, various shells, Node.js
    "$ ",         // Bash/Zsh
    "# ",         // Root shell
    "λ> ",        // Haskell
    "ghci> ",     // Haskell GHCi
    "irb> ",      // Ruby IRB
    "irb(main):", // Ruby IRB with context
    "node> ",     // Node.js
    "julia> ",    // Julia
    "mysql> ",    // MySQL
    "postgres=#", // PostgreSQL
    "sqlite> ",   // SQLite
    "In [",       // IPython/Jupyter (special case)
    "Out[",       // IPython/Jupyter output
];

// ============================================================================
// METRICS
// ============================================================================

/// Metrics for monitoring terminal session health and performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalMetrics {
    /// Total sessions created since server start
    pub total_sessions_created: u64,
    /// Currently active sessions
    pub active_sessions: usize,
    /// Sessions in `completed_sessions` map
    pub completed_sessions: usize,
    /// Average session duration in seconds
    pub average_session_duration_secs: f64,
    /// Maximum concurrent sessions reached
    pub max_concurrent_sessions: usize,
    /// Total commands executed
    pub total_commands_executed: u64,
}

// ============================================================================
// SESSION TYPES
// ============================================================================

/// Terminal session information for internal tracking (active sessions)
#[derive(Clone)]
pub struct TerminalSessionInfo {
    pub pid: u32,
    pub command: String,

    // NEW: Direct terminal reference encapsulates all state
    pub terminal: Arc<RwLock<crate::pty::Terminal>>,

    // KEEP: Activity tracking for auto-cleanup
    pub last_read_time: Arc<RwLock<Instant>>,

    // KEEP: Existing fields unchanged
    pub is_blocked: bool,
    pub ready_for_input: bool,
    pub start_time: DateTime<Utc>,
}

/// Active terminal session information for external API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTerminalSession {
    pub pid: u32,
    pub is_blocked: bool,
    /// Runtime in milliseconds
    pub runtime: u64,
}

/// Response for `get_output` (paginated terminal output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutputResponse {
    /// Process ID
    pub pid: u32,

    /// Lines returned in this page
    pub lines: Vec<String>,

    /// Total lines currently buffered
    pub total_lines: usize,

    /// Number of lines in this response
    pub lines_returned: usize,

    /// Process has finished executing
    pub is_complete: bool,

    /// Exit code (if process completed)
    pub exit_code: Option<i32>,

    /// More output may be available (check again)
    pub has_more: bool,

    /// Indicates if buffer reached size limit (early output may be lost)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer_truncated: Option<bool>,
}

/// Completed terminal session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTerminalSession {
    pub pid: u32,
    pub output: String,
    pub exit_code: Option<i32>,
    pub start_time: std::time::SystemTime,
    pub end_time: std::time::SystemTime,
}

/// Result of terminal command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalCommandResult {
    pub pid: u32,
    pub output: String,
    pub is_blocked: bool,
    pub ready_for_input: bool,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Detect if a REPL is ready for input by checking for known prompt patterns
fn detect_repl_ready(output: &str) -> bool {
    // Get last non-empty line
    let last_line = output
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");

    // Check for exact prompt matches
    if REPL_PROMPTS
        .iter()
        .any(|prompt| last_line.ends_with(prompt))
    {
        return true;
    }

    // Special case for IPython/Jupyter
    if last_line.starts_with("In [") && last_line.contains("]: ") {
        return true;
    }

    false
}

// ============================================================================
// TERMINAL MANAGER
// ============================================================================

/// Terminal manager for handling command execution and session management
#[derive(Clone)]
pub struct TerminalManager {
    sessions: Arc<Mutex<HashMap<u32, TerminalSessionInfo>>>,
    completed_sessions: Arc<Mutex<HashMap<u32, CompletedTerminalSession>>>,
    next_pid: Arc<AtomicU32>,
}

impl TerminalManager {
    /// Create a new terminal manager instance
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            completed_sessions: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(AtomicU32::new(1000)),
        }
    }

    // ========================================================================
    // SPAWN COMMAND - PTY-based implementation
    // ========================================================================

    /// Spawn a new command in a PTY terminal
    ///
    /// This creates a PTY-based terminal with VT100 emulation, enabling:
    /// - Interactive programs (vim, less, top)
    /// - ANSI color sequences
    /// - Proper TTY detection by child processes
    pub async fn spawn_command(
        &self,
        command: &str,
        shell_path: Option<&str>,
    ) -> Result<u32, anyhow::Error> {
        // 1. Build PTY terminal OUTSIDE lock (slow operation)
        let mut builder = Terminal::builder()
            .command(command)
            .size(24, 80)
            .scrollback(MAX_OUTPUT_BUFFER_LINES)
            .shell(true);

        if let Some(shell) = shell_path {
            builder = builder.shell_path(shell);
        }

        let mut terminal = builder.build();

        // 2. Initialize PTY OUTSIDE lock (spawns child process and I/O tasks)
        terminal.init().await?;

        // 3. Generate PID (atomic operation, safe outside lock)
        let pid = self.next_pid.fetch_add(1, AtomicOrdering::SeqCst);

        // 4. Create session info
        let session = TerminalSessionInfo {
            pid,
            command: command.to_string(),
            terminal: Arc::new(RwLock::new(terminal)),
            last_read_time: Arc::new(RwLock::new(Instant::now())),
            is_blocked: false,
            ready_for_input: false,
            start_time: Utc::now(),
        };

        // 5. ATOMIC: Check limit and insert in SINGLE lock scope
        let mut sessions = self.sessions.lock().await;
        if sessions.len() >= MAX_SESSIONS {
            // Must clean up spawned terminal before returning error
            drop(sessions); // Release lock before async close

            let mut terminal = session.terminal.write().await;
            if let Err(e) = terminal.close().await {
                log::error!("Failed to clean up terminal after MAX_SESSIONS reached: {e}");
            }

            return Err(anyhow::anyhow!(
                "Maximum session limit reached ({MAX_SESSIONS}/{MAX_SESSIONS} sessions). \
                 Please wait for existing sessions to complete or stop them manually."
            ));
        }

        // Insert happens in same lock scope as check - ATOMIC
        sessions.insert(pid, session);
        log::info!(
            "Session spawned: pid={}, command={}, active={}/{}",
            pid,
            command,
            sessions.len(),
            MAX_SESSIONS
        );

        Ok(pid)
    }

    // ========================================================================
    // EXECUTE COMMAND - Simplified wrapper using spawn_command
    // ========================================================================

    /// Execute a command in a new terminal session
    ///
    /// # Errors
    /// Returns error if command execution fails, process cannot be spawned, or I/O errors occur
    pub async fn execute_command(
        &self,
        command: &str,
        initial_delay_ms: Option<u64>,
        shell: Option<&str>,
    ) -> Result<TerminalCommandResult, anyhow::Error> {
        // Use spawn_command which creates PTY terminal
        let pid = self.spawn_command(command, shell).await?;

        // Wait for initial delay to capture quick output (pwd, echo, etc.)
        let delay = Duration::from_millis(initial_delay_ms.unwrap_or(100));
        sleep(delay).await;

        // Get initial output if available
        let output_response = self.get_output(pid, 0, 1000).await;

        let (output, is_blocked, ready_for_input) = if let Some(resp) = output_response {
            let text = resp.lines.join("");
            let ready = detect_repl_ready(&text);
            (text, !ready, ready)
        } else {
            (String::new(), false, false)
        };

        Ok(TerminalCommandResult {
            pid,
            output,
            is_blocked,
            ready_for_input,
        })
    }

    // ========================================================================
    // GET OUTPUT - Paginated output from VT100 screen buffer
    // ========================================================================

    /// Get paginated output from a running command
    ///
    /// Extracts text from the VT100 screen buffer with pagination support.
    ///
    /// # Parameters
    /// - `pid`: Process ID
    /// - `offset`: Starting line (negative = tail from end)
    /// - `length`: Maximum lines to return
    ///
    /// # Returns
    /// Terminal output with pagination info, or None if session not found
    pub async fn get_output(
        &self,
        pid: u32,
        offset: i64,
        length: usize,
    ) -> Option<TerminalOutputResponse> {
        // 1. Get session
        let sessions = self.sessions.lock().await;
        let session = sessions.get(&pid)?;

        // 2. Get screen, calculate pagination, and collect ONLY requested range
        let (lines, total_lines, start, end, is_complete) = {
            let terminal = session.terminal.read().await;
            let screen = terminal.screen()?;
            let (_rows, cols) = screen.size();

            // Count total lines in scrollback buffer (NOT viewport size)
            // screen.size() returns viewport (24x80), not actual buffer size
            // Must iterate to count, but this is cheap (no string allocation)
            let total = screen.rows(0, cols).count();

            // Calculate pagination range
            let (start, end) = if offset < 0 {
                // Negative offset: tail behavior (last N lines)
                let tail_count = usize::try_from(-offset).unwrap_or(0).min(total);
                let start_pos = total.saturating_sub(tail_count);
                (start_pos, total)
            } else {
                // Positive offset: range read (offset..offset+length)
                let start_pos = usize::try_from(offset).unwrap_or(0).min(total);
                let end_pos = (start_pos + length).min(total);
                (start_pos, end_pos)
            };

            // Collect ONLY the requested range (massive memory savings)
            let lines: Vec<String> = screen.rows(0, cols).skip(start).take(end - start).collect();

            let complete = terminal.is_pty_closed();
            (lines, total, start, end, complete)
        }; // Read lock automatically dropped here

        // 4. Get exit code with write lock (separate, minimal critical section)
        let exit_code = if is_complete {
            let mut terminal = session.terminal.write().await;
            terminal
                .try_wait()
                .await
                .ok()
                .flatten()
                .map(|status| i32::from(!status.success()))
        } else {
            None
        };

        let has_more = end < total_lines || !is_complete;

        // 7. Update last read time
        *session.last_read_time.write().await = Instant::now();

        Some(TerminalOutputResponse {
            pid,
            lines,
            total_lines,
            lines_returned: end - start,
            is_complete,
            exit_code,
            has_more,
            buffer_truncated: Some(false), // VT100 scrollback handles truncation
        })
    }

    // ========================================================================
    // SEND INPUT - Interactive input to PTY
    // ========================================================================

    /// Send input to a running command
    ///
    /// Sends text to the PTY with optional newline appending.
    ///
    /// # Parameters
    /// - `pid`: Process ID
    /// - `input`: Text to send
    /// - `append_newline`: If true, appends '\n' to execute command (default: true)
    ///
    /// # Returns
    /// Ok(true) if successful, Err if session not found
    pub async fn send_input(
        &self,
        pid: u32,
        input: &str,
        append_newline: bool,
    ) -> Result<bool, anyhow::Error> {
        // 1. Get session (clone to release lock quickly)
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(&pid)
            .ok_or_else(|| anyhow::anyhow!("Process {pid} not found"))?
            .clone();
        drop(sessions); // Release lock before async PTY call

        // 2. Send to PTY terminal with conditional newline
        let terminal = session.terminal.read().await;

        let bytes = if append_newline {
            // Avoid intermediate String allocation from format!
            let mut buf = Vec::with_capacity(input.len() + 1);
            buf.extend_from_slice(input.as_bytes());
            buf.push(b'\n');
            Bytes::from(buf)
        } else {
            // Direct copy without intermediate Vec allocation
            Bytes::copy_from_slice(input.as_bytes())
        };

        terminal.send_input(bytes).await?;
        drop(terminal);

        log::debug!("Input sent: pid={}, bytes={}", pid, input.len());

        // 3. Update session state
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(&pid) {
            session.is_blocked = false;
            session.ready_for_input = false;
        }

        Ok(true)
    }

    // ========================================================================
    // FORCE TERMINATE - PTY-based termination
    // ========================================================================

    /// Force terminate a running command
    ///
    /// Kills the PTY child process and waits for cleanup.
    /// Uses `terminal.close()` which handles graceful SIGTERM → SIGKILL escalation.
    ///
    /// # Parameters
    /// - `pid`: Process ID to terminate
    ///
    /// # Errors
    /// - `McpError::InvalidArguments`: Session not found
    /// - `McpError::Other`: Terminal close failed
    pub async fn force_terminate(&self, pid: u32) -> Result<(), McpError> {
        // 1. Get session
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(&pid)
            .ok_or_else(|| {
                McpError::InvalidArguments(format!("No active session found for PID: {pid}"))
            })?
            .clone();
        drop(sessions);

        // 2. Close terminal (kills child, waits for tasks)
        let mut terminal = session.terminal.write().await;
        terminal.close().await.map_err(|e| {
            McpError::Other(anyhow::anyhow!(
                "Failed to close terminal for PID {pid}: {e}"
            ))
        })?;

        log::warn!(
            "Session terminated: pid={}, runtime={}s",
            pid,
            (Utc::now() - session.start_time).num_seconds()
        );
        Ok(())
    }

    // ========================================================================
    // GET SESSION - Full implementation from src2
    // ========================================================================

    /// Get a session by PID, returns the PID if session exists
    pub async fn get_session(&self, pid: u32) -> Option<u32> {
        let sessions_guard = self.sessions.lock().await;
        if sessions_guard.contains_key(&pid) {
            Some(pid)
        } else {
            None
        }
    }

    // ========================================================================
    // LIST ACTIVE SESSIONS - Full implementation from src2
    // ========================================================================

    /// List all active terminal sessions
    #[must_use]
    pub fn list_active_sessions(&self) -> Vec<ActiveTerminalSession> {
        if let Ok(sessions_guard) = self.sessions.try_lock() {
            let now = Utc::now();
            sessions_guard
                .values()
                .map(|session| {
                    // Convert runtime to milliseconds, clamping negative to 0
                    let runtime_ms = (now - session.start_time).num_milliseconds();
                    let runtime = u64::try_from(runtime_ms).unwrap_or(0);
                    ActiveTerminalSession {
                        pid: session.pid,
                        is_blocked: session.is_blocked,
                        runtime,
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    // ========================================================================
    // LIST COMPLETED SESSIONS - Full implementation from src2
    // ========================================================================

    /// List all completed terminal sessions
    #[must_use]
    pub fn list_completed_sessions(&self) -> Vec<CompletedTerminalSession> {
        if let Ok(completed_guard) = self.completed_sessions.try_lock() {
            completed_guard.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    // ========================================================================
    // METRICS - Get metrics for monitoring terminal session health
    // ========================================================================

    /// Get metrics for monitoring terminal session health
    ///
    /// Returns statistics about session usage, helping identify:
    /// - Memory leaks (`active_sessions` growing)
    /// - Cleanup issues (`completed_sessions` not being cleared)
    /// - Performance problems (average duration increasing)
    pub async fn metrics(&self) -> TerminalMetrics {
        let sessions = self.sessions.lock().await;
        let completed = self.completed_sessions.lock().await;

        // Calculate average session duration from completed sessions
        let mut total_duration_secs = 0.0;
        let mut count = 0;

        for session in completed.values() {
            if let Ok(duration) = session.end_time.duration_since(session.start_time) {
                total_duration_secs += duration.as_secs_f64();
                count += 1;
            }
        }

        let average_duration = if count > 0 {
            total_duration_secs / f64::from(count)
        } else {
            0.0
        };

        // Get total sessions created from atomic counter
        let total_created = u64::from(self.next_pid.load(AtomicOrdering::SeqCst)) - 1000;

        TerminalMetrics {
            total_sessions_created: total_created,
            active_sessions: sessions.len(),
            completed_sessions: completed.len(),
            average_session_duration_secs: average_duration,
            max_concurrent_sessions: sessions.len(), // Current as proxy (could track separately)
            total_commands_executed: total_created,
        }
    }

    // ========================================================================
    // CLEANUP SESSIONS - Session cleanup to prevent unbounded memory growth
    // ========================================================================

    /// Clean up old completed sessions with differentiated retention.
    ///
    /// Moves completed sessions to `completed_sessions` `HashMap` before removing.
    /// Explicitly closes terminals to prevent resource leaks.
    /// Retention policy:
    /// - Completed sessions: 30 seconds retention in active, then 5 minutes in completed
    /// - Active sessions: 5 minutes retention
    pub async fn cleanup_sessions(&self) {
        let now = Instant::now();

        // Calculate different cutoff times
        let active_cutoff = now
            .checked_sub(Duration::from_secs(ACTIVE_SESSION_RETENTION_SECS))
            .unwrap_or(now);

        let completed_cutoff = now
            .checked_sub(Duration::from_secs(COMPLETED_SESSION_RETENTION_SECS))
            .unwrap_or(now);

        let mut sessions = self.sessions.lock().await;
        let initial_count = sessions.len();

        // Collect sessions to remove - can't call async close() inside retain()
        let mut to_remove = Vec::new(); // Active sessions to just remove
        let mut to_complete = Vec::new(); // Completed sessions to move

        for (pid, session) in sessions.iter() {
            // Check completion status from terminal
            let is_complete = session
                .terminal
                .try_read()
                .map(|t| t.is_pty_closed())
                .unwrap_or(false);

            let last_read = session.last_read_time.try_read().map(|t| *t).unwrap_or(now);

            // Differentiated retention based on completion status
            let should_keep = if is_complete {
                // Completed sessions: shorter retention (30 seconds)
                last_read > completed_cutoff
            } else {
                // Active sessions: longer retention (5 minutes)
                last_read > active_cutoff
            };

            if !should_keep {
                if is_complete {
                    to_complete.push((*pid, session.clone()));
                } else {
                    to_remove.push((*pid, session.clone()));
                }
            }
        }

        // Close terminals BEFORE removing from HashMap (prevents resource leaks)
        for (pid, session) in &to_remove {
            log::debug!("Closing terminal for inactive session PID {pid}");
            let mut terminal = session.terminal.write().await;
            if let Err(e) = terminal.close().await {
                log::error!("Failed to close terminal for PID {pid}: {e}");
            }
        }

        // Close terminals for completed sessions too
        for (pid, session) in &to_complete {
            log::debug!("Closing terminal for completed session PID {pid}");
            let mut terminal = session.terminal.write().await;
            if let Err(e) = terminal.close().await {
                log::error!("Failed to close terminal for PID {pid}: {e}");
            }
        }

        // Now safe to remove from HashMap
        for (pid, _) in to_remove {
            sessions.remove(&pid);
        }

        // Move completed sessions to completed_sessions HashMap
        drop(sessions); // Release lock before acquiring completed_sessions lock

        if !to_complete.is_empty() {
            let moved_count = to_complete.len();
            let mut completed = self.completed_sessions.lock().await;

            // Remove from active sessions first
            let mut sessions = self.sessions.lock().await;
            for (pid, session) in to_complete {
                sessions.remove(&pid);

                // Get final exit code if available
                let exit_code = {
                    let mut terminal = session.terminal.write().await;
                    terminal
                        .try_wait()
                        .await
                        .ok()
                        .flatten()
                        .map(|status| i32::from(!status.success()))
                };

                // Get final output from VT100 screen buffer
                let output = {
                    let terminal = session.terminal.read().await;
                    if let Some(screen) = terminal.screen() {
                        let (_rows, cols) = screen.size();
                        let lines: Vec<String> = screen.rows(0, cols).collect();
                        lines.join("\n")
                    } else {
                        String::new()
                    }
                };

                // Convert timestamps (session.start_time is DateTime<Utc>, need SystemTime)
                let start_time = {
                    let duration_secs = (Utc::now() - session.start_time).num_seconds();
                    if duration_secs > 0 {
                        std::time::SystemTime::now()
                            .checked_sub(Duration::from_secs(duration_secs as u64))
                            .unwrap_or_else(std::time::SystemTime::now)
                    } else {
                        // Clock skew: start_time is in future, use current time
                        std::time::SystemTime::now()
                    }
                };

                let end_time = std::time::SystemTime::now();

                // Create completed session record
                let completed_session = CompletedTerminalSession {
                    pid,
                    output,
                    exit_code,
                    start_time,
                    end_time,
                };

                completed.insert(pid, completed_session);
            }

            log::info!("Moved {moved_count} sessions to completed_sessions");
        }

        let cleaned_count = initial_count - self.sessions.lock().await.len();
        if cleaned_count > 0 {
            log::info!(
                "Session cleanup: removed={}, active={}, completed={}",
                cleaned_count,
                self.sessions.lock().await.len(),
                self.completed_sessions.lock().await.len()
            );
        }
    }

    /// Clean up old completed sessions (older than 5 minutes)
    ///
    /// Called periodically by cleanup task to prevent unbounded memory growth.
    async fn cleanup_completed_sessions(&self) {
        let now = std::time::SystemTime::now();
        let cutoff = Duration::from_secs(5 * 60); // 5 minutes

        let mut completed = self.completed_sessions.lock().await;
        let initial_count = completed.len();

        completed.retain(|pid, session| {
            let age = now
                .duration_since(session.end_time)
                .unwrap_or(Duration::ZERO);
            let should_keep = age < cutoff;

            if !should_keep {
                log::debug!("Removing old completed session PID {pid} (age: {age:?})");
            }

            should_keep
        });

        let removed_count = initial_count - completed.len();
        if removed_count > 0 {
            log::info!("Cleaned up {removed_count} old completed sessions");
        }
    }

    /// Start background cleanup task (call once at server startup).
    ///
    /// Spawns a tokio task that runs cleanup every minute with differentiated retention:
    /// - Active sessions: 5 minutes retention
    /// - Completed sessions: 30 seconds retention
    ///
    /// # Pattern
    /// Follows the same pattern as `sequential_thinking` cleanup:
    /// packages/sequential-thinking/src/sequential_thinking.rs:353-363
    ///
    /// # Usage
    /// Called from main.rs after wrapping manager in Arc:
    /// ```rust,no_run
    /// use kodegen_terminal::TerminalManager;
    /// use std::sync::Arc;
    ///
    /// let terminal_manager = TerminalManager::new();
    /// let terminal_manager_arc = Arc::new(terminal_manager.clone());
    /// terminal_manager_arc.start_cleanup_task();
    /// ```
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
            loop {
                interval.tick().await;

                // Clean up both active and completed sessions
                self.cleanup_sessions().await;
                self.cleanup_completed_sessions().await;
            }
        });
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}
