use std::fs;
use std::time::Duration;

use anyhow::{Context, Result};
use log::info;
use tokio::process::Command;
use tokio::time::timeout;

const GIT_CLONE_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes

/// Install fluent voice from git repository
pub async fn install_fluent_voice(fluent_voice_dir: &std::path::Path) -> Result<()> {
    clone_from_git(fluent_voice_dir).await
}

/// Uninstall fluent voice components by removing the installation directory
pub async fn uninstall_fluent_voice(fluent_voice_dir: &std::path::Path) -> Result<()> {
    if !fluent_voice_dir.exists() {
        info!("Fluent voice directory does not exist, nothing to uninstall");
        return Ok(());
    }

    info!("Removing fluent-voice directory: {fluent_voice_dir:?}");

    fs::remove_dir_all(fluent_voice_dir).with_context(|| {
        format!("Failed to remove fluent-voice directory: {fluent_voice_dir:?}")
    })?;

    info!("Successfully uninstalled fluent-voice components");
    Ok(())
}

/// Clone from git repository with retries
async fn clone_from_git(fluent_voice_dir: &std::path::Path) -> Result<()> {
    const MAX_RETRIES: u32 = 3;
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        if attempt > 1 {
            info!("Retrying git clone (attempt {attempt}/{MAX_RETRIES})");
            // Brief delay before retry
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        let output = match timeout(
            GIT_CLONE_TIMEOUT,
            Command::new("git")
                .args([
                    "clone",
                    "https://github.com/kodegen/fluent-voice.git",
                    fluent_voice_dir.to_str().ok_or_else(|| {
                        anyhow::anyhow!("fluent-voice directory path contains invalid UTF-8")
                    })?,
                ])
                .output(),
        )
        .await
        {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                last_error = Some(format!("Git command failed: {e}"));

                // Clean up failed attempt
                if fluent_voice_dir.exists() {
                    let _ = fs::remove_dir_all(fluent_voice_dir);
                }
                continue;
            }
            Err(_) => {
                last_error = Some(format!(
                    "Git clone timed out after {} seconds",
                    GIT_CLONE_TIMEOUT.as_secs()
                ));

                // Clean up failed attempt
                if fluent_voice_dir.exists() {
                    let _ = fs::remove_dir_all(fluent_voice_dir);
                }
                continue;
            }
        };

        if output.status.success() {
            info!("Successfully cloned fluent-voice repository");
            return Ok(());
        }

        let error_msg = String::from_utf8_lossy(&output.stderr);
        last_error = Some(error_msg.to_string());

        // Clean up failed attempt
        if fluent_voice_dir.exists() {
            let _ = fs::remove_dir_all(fluent_voice_dir);
        }
    }

    Err(anyhow::anyhow!(
        "Failed to clone fluent-voice after {} attempts. Last error: {}",
        MAX_RETRIES,
        last_error.unwrap_or_else(|| "Unknown error".to_string())
    ))
}
