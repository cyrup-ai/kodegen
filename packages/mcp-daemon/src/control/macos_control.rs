//! macOS daemon control using launchd (launchctl)

use anyhow::{Context, Result};
use std::process::Command;
use std::time::Duration;

const SERVICE_LABEL: &str = "ai.kodegen.kodegend";
const PLIST_PATH: &str = "/Library/LaunchDaemons/kodegend.plist";

/// Check if daemon is running via launchctl list
///
/// Returns: Ok(true) if service is loaded and running, Ok(false) otherwise
pub fn check_status() -> Result<bool> {
    let output = Command::new("launchctl")
        .args(["list", SERVICE_LABEL])
        .output()
        .context("Failed to execute launchctl list")?;

    // launchctl list returns:
    // - Exit 0 if service is loaded (may be running or stopped)
    // - Exit 1 if service not found
    
    if !output.status.success() {
        return Ok(false); // Service not loaded
    }

    // Parse output to check if PID exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Output format: "PID\tStatus\tLabel"
    // If PID is "-", service is loaded but not running
    // If PID is a number, service is running
    for line in stdout.lines() {
        if line.contains(SERVICE_LABEL) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pid) = parts.first() {
                return Ok(*pid != "-");
            }
        }
    }

    Ok(false)
}

/// Start daemon via launchctl
///
/// Uses modern kickstart command with legacy load fallback
pub fn start_daemon() -> Result<()> {
    // Try modern bootstrap first (may fail if already loaded - that's OK)
    let _ = Command::new("launchctl")
        .args(["bootstrap", "system", PLIST_PATH])
        .output();

    // Then kickstart to ensure it starts
    let output = Command::new("launchctl")
        .args(["kickstart", SERVICE_LABEL])
        .output()
        .context("Failed to execute launchctl kickstart")?;

    if !output.status.success() {
        // Fallback to legacy load command
        let load_output = Command::new("launchctl")
            .args(["load", "-w", PLIST_PATH])
            .output()
            .context("Failed to execute launchctl load")?;

        if !load_output.status.success() {
            anyhow::bail!(
                "Failed to start daemon: {}",
                String::from_utf8_lossy(&load_output.stderr)
            );
        }
    }

    Ok(())
}

/// Stop daemon via launchctl
///
/// Uses modern kill + bootout with legacy unload fallback
pub fn stop_daemon() -> Result<()> {
    // Try to kill the service first (graceful shutdown)
    let _ = Command::new("launchctl")
        .args(["kill", "SIGTERM", SERVICE_LABEL])
        .output();

    // Give it a moment to shutdown gracefully
    std::thread::sleep(Duration::from_millis(500));

    // Then bootout
    let output = Command::new("launchctl")
        .args(["bootout", "system", PLIST_PATH])
        .output()
        .context("Failed to execute launchctl bootout")?;

    if !output.status.success() {
        // Fallback to legacy unload
        let unload_output = Command::new("launchctl")
            .args(["unload", "-w", PLIST_PATH])
            .output()
            .context("Failed to execute launchctl unload")?;

        if !unload_output.status.success() {
            anyhow::bail!(
                "Failed to stop daemon: {}",
                String::from_utf8_lossy(&unload_output.stderr)
            );
        }
    }

    Ok(())
}

/// Restart daemon via launchctl
///
/// Uses kickstart -k (kill flag) with manual stop+start fallback
pub fn restart_daemon() -> Result<()> {
    // macOS launchctl doesn't have a direct restart command
    // Use kickstart with -k (kill) flag which restarts the service
    
    let output = Command::new("launchctl")
        .args(["kickstart", "-k", SERVICE_LABEL])
        .output()
        .context("Failed to execute launchctl kickstart -k")?;

    if !output.status.success() {
        // Fallback: manual stop + start
        stop_daemon()?;
        std::thread::sleep(Duration::from_secs(1));
        start_daemon()?;
    }

    Ok(())
}
