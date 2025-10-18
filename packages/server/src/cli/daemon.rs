use tokio::process::Command;
use tokio::time::Duration;

/// Daemon-specific errors with actionable messages
#[derive(Debug)]
pub enum DaemonError {
    NotInstalled,
    NotRunning,
    StartFailed(String),
    Timeout,
    PermissionDenied,
    Other(String),
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInstalled => {
                write!(f, "kodegend daemon is not installed.\n\
                          Install with: cargo install kodegend")
            }
            Self::NotRunning => {
                write!(f, "kodegend daemon is not running.\n\
                          Start with: kodegend start")
            }
            Self::StartFailed(msg) => {
                write!(f, "Failed to start kodegend daemon: {}\n\
                          Check logs with: kodegend logs", msg)
            }
            Self::Timeout => {
                write!(f, "Timeout waiting for kodegend to start.\n\
                          The daemon may be starting slowly or failing to start.\n\
                          Try: kodegend run --foreground")
            }
            Self::PermissionDenied => {
                write!(f, "Permission denied accessing kodegend daemon.\n\
                          You may need to run with sudo or check socket permissions.")
            }
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for DaemonError {}

/// Ensure the kodegend daemon is running before starting stdio mode
///
/// Uses idempotent start strategy to eliminate TOCTOU race conditions:
/// - Always calls `kodegend start` (no separate check)
/// - Treats "already running" as success
/// - Thread-safe: Multiple concurrent calls will all succeed
///
/// Race-free design:
/// - No check-then-start gap
/// - Daemon binary handles concurrent start attempts  
/// - All callers connect to same daemon instance
///
/// Flow:
/// 1. Attempt to start daemon (idempotent operation)
/// 2. If success or "already running" → proceed
/// 3. Poll with two-phase strategy until ready or timeout
pub async fn ensure_daemon_running() -> Result<(), DaemonError> {
    // Always call start - it should be idempotent
    let start = Command::new("kodegend")
        .arg("start")
        .output()  // Use output() to capture stderr
        .await
        .map_err(daemon_error)?;

    if start.status.success() {
        log::info!("kodegend daemon started or already running");
        wait_for_daemon_ready().await?;
        return Ok(());
    }

    // Check if failure was "already running"
    let stderr = String::from_utf8_lossy(&start.stderr);
    if stderr.contains("already running") || stderr.contains("AlreadyRunning") {
        log::info!("kodegend daemon is already running");
        wait_for_daemon_ready().await?;
        return Ok(());
    }

    // Real failure
    Err(DaemonError::StartFailed(stderr.to_string()))
}

/// Check daemon status
///
/// Returns Ok(ExitStatus) if check succeeded, Err if daemon binary not found or check failed
#[inline]
async fn check_daemon_status() -> Result<std::process::ExitStatus, DaemonError> {
    Command::new("kodegend")
        .arg("status")
        .status()
        .await
        .map_err(daemon_error)
}

/// Wait for daemon to be ready using two-phase polling
///
/// Phase 1 - Fast Path (0-50ms):
/// - Check immediately, then poll every 1ms for up to 50ms
/// - Optimized for fast-starting daemons (most common case)
/// - Uses tokio::time::sleep(1ms) to yield properly (no CPU spinning)
///
/// Phase 2 - Slow Path (50ms+):
/// - If daemon not ready after 50ms, use exponential backoff
/// - Wait 100ms, 200ms, 400ms, 800ms, 1600ms (5 attempts)
/// - Total timeout: ~3150ms (50ms fast + 3100ms slow)
///
/// Performance:
/// - Fast daemons (<50ms startup): detected in <10ms typically
/// - Slow daemons: same performance as before (~3s timeout)
async fn wait_for_daemon_ready() -> Result<(), DaemonError> {
    // ===== PHASE 1: FAST PATH =====
    // Rapid polling for quickly-starting daemons (most common case)
    // Check every 1ms for first 50ms
    const FAST_INTERVAL_MS: u64 = 1;
    const FAST_DURATION_MS: u64 = 50;
    
    let fast_attempts = FAST_DURATION_MS / FAST_INTERVAL_MS;
    
    for attempt in 1..=fast_attempts {
        if check_daemon_status().await?.success() {
            log::debug!(
                "Daemon ready after {}ms (fast path, attempt {})",
                attempt * FAST_INTERVAL_MS,
                attempt
            );
            return Ok(());
        }
        
        // Yield to scheduler between checks (prevents CPU spinning)
        tokio::time::sleep(Duration::from_millis(FAST_INTERVAL_MS)).await;
    }
    
    // ===== PHASE 2: SLOW PATH =====
    // Daemon didn't start quickly - use exponential backoff
    log::debug!("Daemon not ready after 50ms, switching to exponential backoff");
    
    const SLOW_INITIAL_MS: u64 = 100;
    const SLOW_MAX_MS: u64 = 1600;
    const SLOW_ATTEMPTS: u32 = 5;
    
    let mut backoff_ms = SLOW_INITIAL_MS;
    let mut total_elapsed_ms: u64 = FAST_DURATION_MS;
    
    for attempt in 1..=SLOW_ATTEMPTS {
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        total_elapsed_ms += backoff_ms;
        
        if check_daemon_status().await?.success() {
            log::info!(
                "Daemon started after {} attempt(s) ({}ms total)",
                fast_attempts + u64::from(attempt),
                total_elapsed_ms
            );
            return Ok(());
        }
        
        backoff_ms = (backoff_ms * 2).min(SLOW_MAX_MS);
        
        if attempt == SLOW_ATTEMPTS {
            return Err(DaemonError::Timeout);
        }
    }
    
    Ok(())
}

/// Map I/O errors to helpful daemon error messages
///
/// Provides context-specific error messages for common failure cases
#[inline]
fn daemon_error(e: std::io::Error) -> DaemonError {
    match e.kind() {
        std::io::ErrorKind::NotFound => DaemonError::NotInstalled,
        std::io::ErrorKind::PermissionDenied => DaemonError::PermissionDenied,
        std::io::ErrorKind::ConnectionRefused => DaemonError::NotRunning,
        _ => DaemonError::Other(format!("Failed to communicate with kodegend daemon: {}", e)),
    }
}
