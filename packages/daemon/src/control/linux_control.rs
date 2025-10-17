//! Linux daemon control using systemd (systemctl)

use anyhow::{Context, Result};
use std::process::Command;

const SERVICE_NAME: &str = "kodegend";

/// Check if daemon is running via systemctl is-active
///
/// Returns: Ok(true) if service is active, Ok(false) if inactive
pub fn check_status() -> Result<bool> {
    let args = if is_root() {
        vec!["is-active", &format!("{}.service", SERVICE_NAME)]
    } else {
        vec!["--user", "is-active", &format!("{}.service", SERVICE_NAME)]
    };

    let output = Command::new("systemctl")
        .args(&args)
        .output()
        .context("Failed to execute systemctl is-active")?;

    // systemctl is-active returns:
    // - Exit 0 if active
    // - Exit 3 if inactive
    // - Other codes for other states
    Ok(output.status.success())
}

/// Start daemon via systemctl start
pub fn start_daemon() -> Result<()> {
    let args = if is_root() {
        vec!["start", &format!("{}.service", SERVICE_NAME)]
    } else {
        vec!["--user", "start", &format!("{}.service", SERVICE_NAME)]
    };

    let output = Command::new("systemctl")
        .args(&args)
        .output()
        .context("Failed to execute systemctl start")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to start daemon: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Stop daemon via systemctl stop
pub fn stop_daemon() -> Result<()> {
    let args = if is_root() {
        vec!["stop", &format!("{}.service", SERVICE_NAME)]
    } else {
        vec!["--user", "stop", &format!("{}.service", SERVICE_NAME)]
    };

    let output = Command::new("systemctl")
        .args(&args)
        .output()
        .context("Failed to execute systemctl stop")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to stop daemon: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Restart daemon via systemctl restart
pub fn restart_daemon() -> Result<()> {
    let args = if is_root() {
        vec!["restart", &format!("{}.service", SERVICE_NAME)]
    } else {
        vec!["--user", "restart", &format!("{}.service", SERVICE_NAME)]
    };

    let output = Command::new("systemctl")
        .args(&args)
        .output()
        .context("Failed to execute systemctl restart")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to restart daemon: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Check if running as root
#[inline]
fn is_root() -> bool {
    unsafe { libc::getuid() } == 0
}
