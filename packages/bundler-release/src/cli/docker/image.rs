//! Docker image management for release builds.
//!
//! Handles building and maintaining the builder Docker image used for
//! cross-platform package creation.

use crate::error::{CliError, ReleaseError};
use chrono::{DateTime, Utc};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

/// Docker image name for the release builder container
pub const BUILDER_IMAGE_NAME: &str = "kodegen-release-builder";

/// Timeout for Docker info check (5 seconds)
/// Quick daemon availability check shouldn't take long
pub const DOCKER_INFO_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for Docker image build operations (30 minutes)
/// Image builds can take a long time due to base image downloads, apt updates, etc.
pub const DOCKER_BUILD_TIMEOUT: Duration = Duration::from_secs(1800);

/// Platform-specific Docker startup instructions
#[cfg(target_os = "macos")]
const DOCKER_START_HELP: &str = "Start Docker Desktop from Applications or Spotlight";

#[cfg(target_os = "linux")]
const DOCKER_START_HELP: &str = "Start Docker daemon: sudo systemctl start docker";

#[cfg(target_os = "windows")]
const DOCKER_START_HELP: &str = "Start Docker Desktop from the Start menu";

/// Checks if Docker is installed and the daemon is running.
///
/// # Returns
///
/// * `Ok(())` - Docker is available
/// * `Err` - Docker is not installed or daemon is not running
pub async fn check_docker_available() -> Result<(), ReleaseError> {
    let status_result = timeout(
        DOCKER_INFO_TIMEOUT,
        Command::new("docker")
            .arg("info")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    )
    .await;

    match status_result {
        // Timeout occurred
        Err(_) => Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker info".to_string(),
            reason: format!(
                "Docker daemon check timed out after {} seconds.\n\
                     \n\
                     This usually means Docker is not responding.\n\
                     {}\n\
                     \n\
                     If Docker is running, check: docker ps",
                DOCKER_INFO_TIMEOUT.as_secs(),
                DOCKER_START_HELP
            ),
        })),

        // Command succeeded
        Ok(Ok(status)) if status.success() => Ok(()),

        // Docker command exists but daemon isn't responding
        Ok(Ok(status)) => {
            let exit_code = status.code().unwrap_or(-1);
            Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker info".to_string(),
                reason: format!(
                    "Docker daemon is not responding (exit code: {}).\n\
                     \n\
                     {} \n\
                     \n\
                     If Docker is installed, ensure the daemon is running.\n\
                     If not installed, visit: https://docs.docker.com/get-docker/",
                    exit_code, DOCKER_START_HELP
                ),
            }))
        }

        // Docker command not found - not installed
        Ok(Err(e)) => Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker".to_string(),
            reason: format!(
                "Docker command not found: {}\n\
                     \n\
                     Docker does not appear to be installed.\n\
                     Install from: https://docs.docker.com/get-docker/\n\
                     \n\
                     Platform-specific instructions:\n\
                     • macOS: Install Docker Desktop (includes GUI and CLI)\n\
                     • Linux: Install docker.io (Ubuntu/Debian) or docker-ce (others)\n\
                     • Windows: Install Docker Desktop",
                e
            ),
        })),
    }
}

/// Ensures the builder Docker image is built and up-to-date.
///
/// Checks if the image exists and whether it's stale (Dockerfile modified after image creation).
/// Automatically rebuilds if Dockerfile is newer than image.
///
/// # Arguments
///
/// * `workspace_path` - Path to workspace containing .devcontainer/Dockerfile
/// * `force_rebuild` - If true, rebuild image unconditionally
/// * `runtime_config` - Runtime configuration for output
///
/// # Returns
///
/// * `Ok(())` - Image is ready and up-to-date
/// * `Err` - Failed to build or check image
pub async fn ensure_image_built(
    workspace_path: &Path,
    force_rebuild: bool,
    runtime_config: &crate::cli::RuntimeConfig,
) -> Result<(), ReleaseError> {
    let dockerfile_path = workspace_path.join(".devcontainer/Dockerfile");

    if !dockerfile_path.exists() {
        return Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "check_dockerfile".to_string(),
            reason: format!(
                "Dockerfile not found at: {}\n\
                 \n\
                 To use Docker for cross-platform builds, you need a Dockerfile.\n\
                 The expected location is:\n\
                 {}\n\
                 \n\
                 This Dockerfile provides a Linux container with:\n\
                 • Rust toolchain (matching rust-toolchain.toml)\n\
                 • Wine + .NET 4.0 (for building Windows .msi installers)\n\
                 • NSIS (for building .exe installers)\n\
                 • Tools for .deb, .rpm, and AppImage creation\n\
                 \n\
                 See example and setup guide:\n\
                 https://github.com/cyrup/kodegen/tree/main/.devcontainer",
                dockerfile_path.display(),
                dockerfile_path.display()
            ),
        }));
    }

    // Force rebuild if requested
    if force_rebuild {
        runtime_config.progress("Force rebuilding Docker image (--rebuild-image)...");
        return build_docker_image(workspace_path, runtime_config).await;
    }

    // Check if image exists
    let check_output = timeout(
        Duration::from_secs(10), // Image check should be fast
        Command::new("docker")
            .args(["images", "-q", BUILDER_IMAGE_NAME])
            .output(),
    )
    .await
    .map_err(|_| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker images".to_string(),
            reason: "Docker image check timed out after 10 seconds".to_string(),
        })
    })?
    .map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker images".to_string(),
            reason: e.to_string(),
        })
    })?;

    let image_id = String::from_utf8_lossy(&check_output.stdout)
        .trim()
        .to_string();

    if !image_id.is_empty() && image_id.len() >= 12 {
        // Image exists - check if it's up-to-date
        runtime_config.verbose_println(&format!(
            "Found existing Docker image: {}",
            &image_id[..12.min(image_id.len())]
        ));

        match is_image_up_to_date(&image_id, &dockerfile_path, runtime_config).await {
            Ok(true) => {
                // Check if image is too old (older than 7 days)
                if let Ok(age_days) = get_image_age_days(&image_id).await
                    && age_days > 7
                {
                    runtime_config.warn(&format!(
                        "Docker image is {} days old - rebuilding to get base image updates",
                        age_days
                    ));
                    return build_docker_image(workspace_path, runtime_config).await;
                }

                runtime_config.verbose_println("Docker image is up-to-date");
                return Ok(());
            }
            Ok(false) => {
                runtime_config.warn(&format!(
                    "Docker image {} is outdated (Dockerfile modified since image creation)",
                    BUILDER_IMAGE_NAME
                ));
                runtime_config.progress("Rebuilding Docker image...");
                return build_docker_image(workspace_path, runtime_config).await;
            }
            Err(e) => {
                // If we can't determine staleness, be conservative and rebuild
                runtime_config.warn(&format!(
                    "Could not verify image freshness: {}\nRebuilding to be safe...",
                    e
                ));
                return build_docker_image(workspace_path, runtime_config).await;
            }
        }
    }

    // Image doesn't exist - build it
    runtime_config.progress(&format!(
        "Building {} Docker image (this may take a few minutes)...",
        BUILDER_IMAGE_NAME
    ));
    build_docker_image(workspace_path, runtime_config).await
}

/// Checks if Docker image is up-to-date with current Dockerfile.
///
/// Compares Dockerfile modification time against Docker image creation time.
///
/// # Arguments
///
/// * `image_id` - Docker image ID or tag
/// * `dockerfile_path` - Path to Dockerfile
/// * `runtime_config` - Runtime config for verbose output
///
/// # Returns
///
/// * `Ok(true)` - Image is up-to-date (created after last Dockerfile modification)
/// * `Ok(false)` - Image is stale (Dockerfile modified after image creation)
/// * `Err` - Could not determine staleness
async fn is_image_up_to_date(
    image_id: &str,
    dockerfile_path: &Path,
    runtime_config: &crate::cli::RuntimeConfig,
) -> Result<bool, ReleaseError> {
    // Get image creation timestamp from Docker
    let inspect_output = Command::new("docker")
        .args(["inspect", "-f", "{{.Created}}", image_id])
        .output()
        .await
        .map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("docker inspect {}", image_id),
                reason: e.to_string(),
            })
        })?;

    if !inspect_output.status.success() {
        let stderr = String::from_utf8_lossy(&inspect_output.stderr);
        return Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker inspect".to_string(),
            reason: format!("Failed to inspect image: {}", stderr),
        }));
    }

    let image_created_str = String::from_utf8_lossy(&inspect_output.stdout)
        .trim()
        .to_string();

    // Parse Docker's RFC3339 timestamp
    let image_created_time = DateTime::parse_from_rfc3339(&image_created_str).map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "parse_timestamp".to_string(),
            reason: format!(
                "Invalid timestamp from Docker '{}': {}",
                image_created_str, e
            ),
        })
    })?;

    // Get Dockerfile modification time
    let dockerfile_metadata = std::fs::metadata(dockerfile_path).map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "stat_dockerfile".to_string(),
            reason: format!("Cannot read Dockerfile metadata: {}", e),
        })
    })?;

    let dockerfile_modified = dockerfile_metadata.modified().map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "get_mtime".to_string(),
            reason: format!("Cannot get Dockerfile modification time: {}", e),
        })
    })?;

    let dockerfile_time: DateTime<Utc> = dockerfile_modified.into();
    let image_time: DateTime<Utc> = image_created_time.into();

    // Compare timestamps
    if dockerfile_time > image_time {
        runtime_config.verbose_println(&format!(
            "Dockerfile modified: {} | Image created: {}",
            dockerfile_time.format("%Y-%m-%d %H:%M:%S UTC"),
            image_time.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        Ok(false) // Stale
    } else {
        runtime_config.verbose_println(&format!(
            "Image is up-to-date (created {} after Dockerfile)",
            humanize_duration((image_time - dockerfile_time).num_seconds())
        ));
        Ok(true)
    }
}

/// Builds the Docker image from Dockerfile.
///
/// # Arguments
///
/// * `workspace_path` - Path to workspace root
/// * `runtime_config` - Runtime configuration for output
///
/// # Returns
///
/// * `Ok(())` - Image built successfully
/// * `Err` - Build failed
pub async fn build_docker_image(
    workspace_path: &Path,
    runtime_config: &crate::cli::RuntimeConfig,
) -> Result<(), ReleaseError> {
    let dockerfile_dir = workspace_path.join(".devcontainer");

    runtime_config.progress(&format!("Building Docker image: {}", BUILDER_IMAGE_NAME));

    // Spawn with piped stdout for streaming
    let mut child = Command::new("docker")
        .args([
            "build",
            "--pull",
            "-t",
            BUILDER_IMAGE_NAME,
            "-f",
            "Dockerfile",
            ".",
        ])
        .current_dir(&dockerfile_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker build".to_string(),
                reason: e.to_string(),
            })
        })?;

    // Stream stdout line-by-line
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            runtime_config.indent(&line);
        }
    }

    // Wait with timeout - handle timeout explicitly to kill child
    let status = tokio::time::timeout(DOCKER_BUILD_TIMEOUT, child.wait()).await;

    let status = match status {
        Ok(Ok(status)) => status, // Completed normally
        Ok(Err(e)) => {
            // Wait failed (process error)
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker build".to_string(),
                reason: e.to_string(),
            }));
        }
        Err(_elapsed) => {
            // Timeout occurred - kill the process before returning error
            runtime_config.warn("Docker build timed out, terminating process...");

            // Kill process (SIGKILL)
            if let Err(e) = child.kill().await {
                eprintln!("Warning: Failed to kill docker build process: {}", e);
            }

            // Wait for process to exit and reap zombie (with short timeout)
            let _ = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;

            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker build".to_string(),
                reason: format!(
                    "Docker build timed out after {} minutes.\n\
                     \n\
                     Possible causes:\n\
                     • Slow network connection to Docker registry\n\
                     • Large base image download\n\
                     • Complex Dockerfile with many layers\n\
                     \n\
                     Solutions:\n\
                     • Check network connection\n\
                     • Increase timeout if build is legitimately slow\n\
                     • Optimize Dockerfile (fewer layers, smaller base images)\n\
                     • Use local registry/cache",
                    DOCKER_BUILD_TIMEOUT.as_secs() / 60
                ),
            }));
        }
    };

    if !status.success() {
        return Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker build".to_string(),
            reason: format!(
                "Build failed with exit code: {}",
                status.code().unwrap_or(-1)
            ),
        }));
    }

    runtime_config.success("Docker image built successfully");
    Ok(())
}

/// Convert seconds to human-readable duration
///
/// Handles positive and negative durations, with correct singular/plural forms.
///
/// # Examples
/// - `humanize_duration(1)` → "1 second"
/// - `humanize_duration(90)` → "1 minute"
/// - `humanize_duration(-3600)` → "-1 hour"
fn humanize_duration(seconds: i64) -> String {
    let is_negative = seconds < 0;
    let abs_seconds = seconds.abs();
    let prefix = if is_negative { "-" } else { "" };

    let (value, unit) = if abs_seconds < 60 {
        (
            abs_seconds,
            if abs_seconds == 1 {
                "second"
            } else {
                "seconds"
            },
        )
    } else if abs_seconds < 3600 {
        let mins = abs_seconds / 60;
        (mins, if mins == 1 { "minute" } else { "minutes" })
    } else if abs_seconds < 86400 {
        let hours = abs_seconds / 3600;
        (hours, if hours == 1 { "hour" } else { "hours" })
    } else {
        let days = abs_seconds / 86400;
        (days, if days == 1 { "day" } else { "days" })
    };

    format!("{}{} {}", prefix, value, unit)
}

/// Gets the age of a Docker image in days.
///
/// # Arguments
///
/// * `image_id` - Docker image ID or tag
///
/// # Returns
///
/// * `Ok(days)` - Number of days since image was created
/// * `Err` - Could not determine image age
async fn get_image_age_days(image_id: &str) -> Result<i64, ReleaseError> {
    // Get image creation timestamp from Docker
    let inspect_output = Command::new("docker")
        .args(["inspect", "-f", "{{.Created}}", image_id])
        .output()
        .await
        .map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("docker inspect {}", image_id),
                reason: e.to_string(),
            })
        })?;

    if !inspect_output.status.success() {
        let stderr = String::from_utf8_lossy(&inspect_output.stderr);
        return Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker inspect".to_string(),
            reason: format!("Failed to get image creation time: {}", stderr),
        }));
    }

    let created_str = String::from_utf8_lossy(&inspect_output.stdout)
        .trim()
        .to_string();

    // Parse Docker's RFC3339 timestamp
    let created_time = DateTime::parse_from_rfc3339(&created_str).map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "parse_timestamp".to_string(),
            reason: format!("Invalid timestamp '{}': {}", created_str, e),
        })
    })?;

    let now = Utc::now();
    let created_utc: DateTime<Utc> = created_time.into();

    Ok((now - created_utc).num_days())
}
