use anyhow::Result;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

/// Ensure the kodegend daemon is running before starting stdio mode
///
/// Uses exponential backoff polling (50ms → 1600ms cap) for optimal performance:
/// - Fast detection for quick daemon startups (50ms first check)
/// - Progressive backoff for slower systems (reduces check frequency)
/// - ~3.15s worst-case timeout (6 attempts)
/// - Zero heap allocations in polling loop
///
/// Flow:
/// 1. Check if daemon already running (fast path: returns immediately)
/// 2. If not running, start daemon
/// 3. Poll with exponential backoff until ready or timeout
pub async fn ensure_daemon_running() -> Result<()> {
    // Fast path: Check if daemon is already running
    let status = check_daemon_status().await?;

    if status.success() {
        log::info!("kodegend daemon is already running");
        return Ok(());
    }

    // Daemon not running, attempt to start it
    log::info!("kodegend daemon not running, starting...");
    start_daemon().await?;

    // Wait for daemon to be ready with exponential backoff
    wait_for_daemon_ready().await?;

    Ok(())
}

/// Check daemon status
///
/// Returns Ok(ExitStatus) if check succeeded, Err if daemon binary not found or check failed
#[inline]
async fn check_daemon_status() -> Result<std::process::ExitStatus> {
    Command::new("kodegend")
        .arg("status")
        .status()
        .await
        .map_err(daemon_error)
}

/// Start the daemon
///
/// Returns Ok(()) if daemon started successfully, Err otherwise
#[inline]
async fn start_daemon() -> Result<()> {
    let start = Command::new("kodegend")
        .arg("start")
        .status()
        .await
        .map_err(daemon_error)?;

    if !start.success() {
        anyhow::bail!("Failed to start kodegend daemon");
    }

    Ok(())
}

/// Wait for daemon to be ready using exponential backoff
///
/// Polling strategy:
/// - Attempt 1: wait 50ms, check
/// - Attempt 2: wait 100ms, check
/// - Attempt 3: wait 200ms, check
/// - Attempt 4: wait 400ms, check
/// - Attempt 5: wait 800ms, check
/// - Attempt 6: wait 1600ms, check (timeout if fails)
///
/// Total worst-case: 3150ms (3.15 seconds)
/// Zero allocations: All variables stack-allocated
async fn wait_for_daemon_ready() -> Result<()> {
    const INITIAL_BACKOFF_MS: u64 = 50;
    const MAX_BACKOFF_MS: u64 = 1600;
    const MAX_ATTEMPTS: u32 = 6;

    let mut backoff_ms = INITIAL_BACKOFF_MS;
    let mut total_elapsed_ms: u64 = 0;

    for attempt in 1..=MAX_ATTEMPTS {
        // Exponential backoff: sleep before checking
        sleep(Duration::from_millis(backoff_ms)).await;
        total_elapsed_ms += backoff_ms;

        // Check if daemon is ready
        let check = check_daemon_status().await?;

        if check.success() {
            log::info!(
                "kodegend daemon started successfully after {} attempt{} ({}ms)",
                attempt,
                if attempt == 1 { "" } else { "s" },
                total_elapsed_ms
            );
            return Ok(());
        }

        // Double backoff for next iteration, capped at maximum
        backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);

        // If this was the last attempt, return timeout error
        if attempt == MAX_ATTEMPTS {
            anyhow::bail!(
                "Daemon failed to start after {} attempts ({}ms elapsed). \
                 Check 'kodegend logs' for details.",
                MAX_ATTEMPTS,
                total_elapsed_ms
            );
        }
    }

    Ok(())
}

/// Map I/O errors to helpful daemon error messages
///
/// Provides context-specific error messages for common failure cases
#[inline]
fn daemon_error(e: std::io::Error) -> anyhow::Error {
    if e.kind() == std::io::ErrorKind::NotFound {
        anyhow::anyhow!(
            "kodegend daemon not found.\n\
             Stdio mode requires kodegend to be installed.\n\
             Install: cargo install kodegend\n\
             Or use SSE mode: kodegen --sse 127.0.0.1:8080"
        )
    } else {
        anyhow::anyhow!("Failed to communicate with kodegend daemon: {}", e)
    }
}
