//! Docker container bundler for cross-platform builds.
//!
//! Manages Docker container lifecycle for building packages on platforms
//! other than the host OS.

use super::artifacts::{find_bundle_directory, verify_artifacts};
use super::guard::ContainerGuard;
use super::limits::ContainerLimits;
use super::platform::{platform_emoji, platform_type_to_string};
use crate::bundler::PackageType;
use crate::error::{CliError, ReleaseError};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

/// Timeout for Docker container run operations (20 minutes)
/// Container bundling involves full cargo builds which can be slow
pub const DOCKER_RUN_TIMEOUT: Duration = Duration::from_secs(1200);

/// Docker container bundler for cross-platform builds.
///
/// Manages Docker container lifecycle for building packages on platforms
/// other than the host OS.
#[derive(Debug)]
pub struct ContainerBundler {
    image_name: String,
    workspace_path: PathBuf,
    pub limits: ContainerLimits,
}

impl ContainerBundler {
    /// Creates a container bundler with custom resource limits.
    ///
    /// # Arguments
    ///
    /// * `workspace_path` - Path to the workspace root (will be mounted in container)
    /// * `limits` - Resource limits for the container
    pub fn with_limits(workspace_path: PathBuf, limits: ContainerLimits) -> Self {
        Self {
            image_name: super::image::BUILDER_IMAGE_NAME.to_string(),
            workspace_path,
            limits,
        }
    }

    /// Check if container was killed by OOM via Docker inspect API
    async fn check_container_oom_status(container_name: &str) -> Result<bool, std::io::Error> {
        let output = Command::new("docker")
            .args(["inspect", container_name, "--format", "{{.State.OOMKilled}}"])
            .output()
            .await?;
        
        if !output.status.success() {
            return Ok(false);  // Container doesn't exist or inspect failed
        }
        
        let oom_killed = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_lowercase();
        
        Ok(oom_killed == "true")
    }

    /// Bundles a single platform in a Docker container.
    ///
    /// Runs the bundle command inside the container, which builds binaries
    /// and creates the package artifact.
    ///
    /// # Arguments
    ///
    /// * `platform` - The package type to build
    /// * `build` - Whether to build binaries before bundling
    /// * `release` - Whether to use release mode
    /// * `runtime_config` - Runtime configuration for output
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PathBuf>)` - Paths to created artifacts
    /// * `Err` - Container execution failed
    pub async fn bundle_platform(
        &self,
        platform: PackageType,
        build: bool,
        release: bool,
        runtime_config: &crate::cli::RuntimeConfig,
    ) -> Result<Vec<PathBuf>, ReleaseError> {
        let platform_str = platform_type_to_string(platform);
        
        runtime_config.indent(&format!("{} Building {} package in container...", 
            platform_emoji(platform), 
            platform_str
        ));

        // Generate UUID for both container name AND temp directory
        let build_uuid = Uuid::new_v4();
        let container_name = format!("kodegen-bundle-{}", build_uuid);

        // Create RAII guard to ensure cleanup on failure
        // Guard will automatically call `docker rm -f` when dropped (on error or panic)
        let _guard = ContainerGuard {
            name: container_name.clone(),
        };

        // Resolve workspace path - try canonicalization but fall back to absolute path
        // Docker will resolve symlinks during bind mount anyway
        let workspace_path = self.workspace_path
            .canonicalize()
            .or_else(|_| {
                // Canonicalize failed (likely network mount) - use absolute path
                if self.workspace_path.is_absolute() {
                    Ok(self.workspace_path.clone())
                } else {
                    std::env::current_dir()
                        .map(|cwd| cwd.join(&self.workspace_path))
                        .map_err(|e| std::io::Error::other(
                            format!("Cannot determine current directory: {}", e)
                        ))
                }
            })
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "resolve workspace path".to_string(),
                reason: format!(
                    "Cannot resolve workspace path '{}': {}\n\
                     \n\
                     Ensure the path exists and is accessible.",
                    self.workspace_path.display(),
                    e
                ),
            }))?;

        // SECURITY: Verify it's actually a directory, not a file
        if !workspace_path.is_dir() {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "validate workspace".to_string(),
                reason: format!(
                    "Workspace path is not a directory: {}\n\
                     \n\
                     The bundle command requires a valid Cargo workspace directory.\n\
                     Check that the path points to a directory containing Cargo.toml.",
                    workspace_path.display()
                ),
            }));
        }

        // Ensure target directory exists (idempotent - safe to call even if exists)
        let target_dir = workspace_path.join("target");
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "create target directory".to_string(),
                reason: format!(
                    "Failed to ensure target directory exists: {}\n\
                     Path: {}\n\
                     This directory is required for build outputs.\n\
                     \n\
                     Check that:\n\
                     • You have write permissions to the workspace\n\
                     • The filesystem is not read-only\n\
                     • There's sufficient disk space",
                    e,
                    target_dir.display()
                ),
            }))?;

        // Create isolated temp target directory for this build
        let temp_target_dir = workspace_path.join(format!("target-temp-{}", build_uuid));
        std::fs::create_dir_all(&temp_target_dir)
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "create temporary target directory".to_string(),
                reason: format!("Failed to create {}: {}", temp_target_dir.display(), e),
            }))?;



        // SECURITY: Build secure mount arguments
        // Mount workspace as read-only (prevents source code modification)
        let workspace_mount = format!("{}:/workspace:ro", workspace_path.display());

        // SECURITY: Mount TEMP target directory as read-write (isolates concurrent builds)
        let target_mount = format!("{}:/workspace/target:rw", temp_target_dir.display());

        // Build docker arguments with security constraints
        let mut docker_args = vec![
            "run".to_string(),
            "--name".to_string(),
            container_name.clone(),
            "--rm".to_string(),
            
            // SECURITY: Prevent privilege escalation in container
            "--security-opt".to_string(),
            "no-new-privileges".to_string(),
            
            // SECURITY: Drop all capabilities (container doesn't need special privileges)
            "--cap-drop".to_string(),
            "ALL".to_string(),
            
            // Memory limits
            "--memory".to_string(),
            self.limits.memory.clone(),
            "--memory-swap".to_string(),
            self.limits.memory_swap.clone(),
            
            // CPU limits
            "--cpus".to_string(),
            self.limits.cpus.clone(),
            
            // Process limits
            "--pids-limit".to_string(),
            self.limits.pids_limit.to_string(),
            
            // SECURITY: Mount workspace read-only
            "-v".to_string(),
            workspace_mount,
            
            // SECURITY: Mount target/ read-write for build outputs
            "-v".to_string(),
            target_mount,
            
            // Set working directory
            "-w".to_string(),
            "/workspace".to_string(),
        ];

        // User mapping for file ownership
        //
        // Unix: Map container user to current host UID/GID
        //       This ensures files created in container have correct ownership
        //       Uses users crate to avoid unsafe code and eliminate TOCTOU race conditions
        //
        // Windows: Use default container user from Dockerfile
        //          Windows container security model doesn't use UID/GID mapping
        #[cfg(unix)]
        {
            let uid = users::get_current_uid();
            let gid = users::get_current_gid();
            docker_args.push("--user".to_string());
            docker_args.push(format!("{}:{}", uid, gid));
        }

        // Note: No --user flag on Windows (uses Dockerfile USER)

        // Add image and cargo command
        docker_args.push(self.image_name.clone());
        docker_args.push("cargo".to_string());
        docker_args.push("run".to_string());
        docker_args.push("-p".to_string());
        docker_args.push("kodegen_release".to_string());
        docker_args.push("--".to_string());
        docker_args.push("bundle".to_string());
        docker_args.push("--platform".to_string());
        docker_args.push(platform_str.to_string());

        if build {
            docker_args.push("--build".to_string());
        }
        if release {
            docker_args.push("--release".to_string());
        }

        // Spawn docker process with both stdout/stderr piped for streaming + OOM detection
        let mut child = Command::new("docker")
            .args(&docker_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("docker run {}", docker_args.join(" ")),
                reason: e.to_string(),
            }))?;

        // Spawn background task to capture stderr for OOM detection
        let stderr_handle = child.stderr.take().map(|stderr| tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut captured_lines = Vec::new();
            
            while let Ok(Some(line)) = lines.next_line().await {
                captured_lines.push(line);
            }
            
            captured_lines
        }));

        // Stream stdout in real-time (foreground task)
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Ok(Some(line)) = lines.next_line().await {
                runtime_config.indent(&line);
            }
        }

        // Wait for child process completion with timeout - handle timeout explicitly to kill child
        let status = tokio::time::timeout(DOCKER_RUN_TIMEOUT, child.wait()).await;

        let status = match status {
            Ok(Ok(status)) => status,  // Completed normally
            Ok(Err(e)) => {
                // Wait failed (process error)
                return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                    command: format!("docker run {}", docker_args.join(" ")),
                    reason: e.to_string(),
                }));
            }
            Err(_elapsed) => {
                // Timeout occurred - kill the process before returning error
                runtime_config.warn(&format!(
                    "Docker bundling timed out after {} minutes, terminating...",
                    DOCKER_RUN_TIMEOUT.as_secs() / 60
                ));
                
                // Kill process (SIGKILL)
                if let Err(e) = child.kill().await {
                    eprintln!("Warning: Failed to kill docker run process: {}", e);
                }
                
                // Wait for process to exit and reap zombie (with short timeout)
                let _ = tokio::time::timeout(
                    Duration::from_secs(10),
                    child.wait()
                ).await;
                
                return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                    command: format!("bundle {} in container", platform_str),
                    reason: format!(
                        "Docker bundling timed out after {} minutes.\n\
                         \n\
                         This usually indicates:\n\
                         • Very slow build (large dependency downloads)\n\
                         • System resource constraints\n\
                         • Network issues\n\
                         \n\
                         Try:\n\
                         • Increase container resource limits\n\
                         • Check available system memory/CPU\n\
                         • Use --build flag to reuse cached builds",
                        DOCKER_RUN_TIMEOUT.as_secs() / 60
                    ),
                }));
            }
        };

        // Retrieve captured stderr from background task
        let stderr_lines = if let Some(handle) = stderr_handle {
            handle.await.unwrap_or_default()
        } else {
            Vec::new()
        };

        // Check exit status and stderr for OOM indicators
        if !status.success() {
            // Check exit code 137 (SIGKILL from OOM)
            let exit_code = status.code().unwrap_or(0);
            let is_oom_exit_code = exit_code == 137;

            // Check stderr strings for OOM indicators
            let stderr_str = stderr_lines.join("\n");
            let is_oom_stderr = stderr_str.contains("OOMKilled") 
                || stderr_str.contains("out of memory")
                || stderr_str.contains("Out of memory")  // Case variation
                || stderr_str.contains("OutOfMemoryError")
                || stderr_str.contains("Cannot allocate memory")
                || stderr_str.to_lowercase().contains("oom");  // Catch all variants

            // Check Docker container status (most reliable method)
            let is_oom_status = Self::check_container_oom_status(&container_name).await.unwrap_or(false);

            let is_oom = is_oom_exit_code || is_oom_stderr || is_oom_status;

            if is_oom {
                // Get system memory info
                let mut sys = sysinfo::System::new();
                sys.refresh_memory();
                let total_memory_gb = sys.total_memory() / 1024 / 1024 / 1024;

                let mut reason = String::from("Container ran out of memory during build.\n\n");
                
                // Add detection method for debugging
                if is_oom_status {
                    reason.push_str("(Detected via Docker container status)\n");
                } else if is_oom_exit_code {
                    reason.push_str("(Detected via exit code 137 - SIGKILL)\n");
                } else {
                    reason.push_str("(Detected via error message)\n");
                }

                reason.push_str(&format!(
                    "\nCurrent memory limit: {} (swap: {})\n\
                     \n\
                     The container exhausted available memory while building. This typically happens when:\n\
                     • Building large Rust projects with many dependencies\n\
                     • Parallel compilation uses more RAM than available\n\
                     • Debug builds require more memory than release builds\n\
                     \n\
                     Solutions:\n\
                     1. Increase memory limit:\n\
                        cargo run -p kodegen_release -- bundle --platform {} --docker-memory 8g\n\
                     \n\
                     2. Build fewer platforms in parallel (run multiple times with --platform)\n\
                     \n\
                     3. Use release builds (they use less memory):\n\
                        cargo run -p kodegen_release -- bundle --platform {} --release\n\
                     \n\
                     4. Check available system memory: {} GB total",
                    self.limits.memory,
                    self.limits.memory_swap,
                    platform_str,
                    platform_str,
                    total_memory_gb,
                ));

                return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                    command: format!("bundle {} in container", platform_str),
                    reason,
                }));
            } else {
                // Generic failure with captured output
                let error_output = if !stderr_str.is_empty() {
                    // Note: stdout already streamed, no need to include it in error
                    format!("stderr:\n{}", stderr_str)
                } else {
                    "No error output captured".to_string()
                };

                return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                    command: format!("bundle {} in container", platform_str),
                    reason: format!(
                        "Container bundling failed with exit code: {}\n\n{}",
                        status.code().unwrap_or(-1),
                        error_output
                    ),
                }));
            }
        }

        runtime_config.indent(&format!("✓ Created {} package", platform_str));

        // Find created artifacts using case-insensitive directory search
        let bundle_dir = find_bundle_directory(&temp_target_dir, platform_str)?;

        // Collect artifact paths with proper error handling
        runtime_config.verbose_println(&format!("Scanning for artifacts in: {}", bundle_dir.display()));

        let entries = std::fs::read_dir(&bundle_dir)
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "read bundle directory".to_string(),
                reason: format!("Failed to read {}: {}", bundle_dir.display(), e),
            }))?;

        let mut artifacts = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "read directory entry".to_string(),
                reason: format!("Failed to read entry in {}: {}", bundle_dir.display(), e),
            }))?;
            let path = entry.path();
            
            // Skip non-regular files (directories, symlinks)
            let metadata = std::fs::symlink_metadata(&path).map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "read file metadata".to_string(),
                reason: format!("Failed to read metadata for {}: {}", path.display(), e),
            }))?;
            if !metadata.is_file() || metadata.is_symlink() {
                runtime_config.verbose_println(&format!("  Skipping non-regular file: {}", path.display()));
                continue;
            }
            
            // Check minimum size
            if metadata.len() < 1024 {
                runtime_config.verbose_println(&format!("  Skipping small file: {} ({} bytes)", path.display(), metadata.len()));
                continue;
            }
            
            // Validate file extension matches platform
            let extension = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());
            
            let is_valid_artifact = match platform {
                PackageType::Deb => extension.as_deref() == Some("deb"),
                PackageType::Rpm => extension.as_deref() == Some("rpm"),
                PackageType::AppImage => {
                    extension.is_none() || extension.as_deref() == Some("appimage")
                }
                PackageType::WindowsMsi => extension.as_deref() == Some("msi"),
                PackageType::Nsis => extension.as_deref() == Some("exe"),
                PackageType::Dmg => extension.as_deref() == Some("dmg"),
                PackageType::MacOsBundle => extension.as_deref() == Some("app"),
            };
            
            if is_valid_artifact {
                runtime_config.verbose_println(&format!("  ✓ Artifact: {}", path.display()));
                artifacts.push(path);
            } else {
                runtime_config.verbose_println(&format!("  Skipping non-artifact: {} (wrong extension)", path.display()));
            }
        }

        runtime_config.verbose_println(&format!("Collected {} artifact(s)", artifacts.len()));

        if artifacts.is_empty() {
            // Show what we found instead of just saying "nothing found"
            let dir_contents = match std::fs::read_dir(&bundle_dir) {
                Ok(entries) => {
                    let items: Vec<_> = entries
                        .flatten()  // OK here since it's just diagnostic info
                        .map(|e| {
                            let path = e.path();
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("<unknown>");
                            if path.is_dir() {
                                format!("  [DIR]  {}", name)
                            } else {
                                let size = path.metadata()
                                    .ok()
                                    .map(|m| m.len())
                                    .unwrap_or(0);
                                format!("  [FILE] {} ({} bytes)", name, size)
                            }
                        })
                        .collect();
                    if items.is_empty() {
                        None
                    } else {
                        Some(items.join("\n"))
                    }
                }
                Err(e) => {
                    Some(format!("[Cannot read directory: {}]", e))
                }
            };
            
            let reason = match dir_contents {
                Some(contents) => format!(
                    "No artifact files found matching expected patterns in:\n\
                     {}\n\
                     \n\
                     Directory contents:\n\
                     {}\n\
                     \n\
                     Expected artifacts like:\n\
                     • {}.deb (Debian package)\n\
                     • {}.rpm (RedHat package)\n\
                     • {}.AppImage (AppImage bundle)\n\
                     etc.",
                    bundle_dir.display(),
                    contents,
                    platform_str,
                    platform_str,
                    platform_str
                ),
                None => format!(
                    "Bundle directory is empty or inaccessible:\n\
                     {}\n\
                     \n\
                     Possible causes:\n\
                     • Bundle command failed silently inside container\n\
                     • Incorrect output directory path\n\
                     • Permission issues\n\
                     \n\
                     Check container logs:\n\
                     docker ps -a | head -2",
                    bundle_dir.display()
                ),
            };
            
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "find artifacts".to_string(),
                reason,
            }));
        }

        // Verify artifacts are valid before declaring success
        verify_artifacts(&artifacts, runtime_config)?;

        // Atomically move artifacts from temp to final location
        // This is the ONLY point where race conditions could occur, but rename is atomic
        let final_bundle_dir = self.workspace_path
            .join("target")
            .join("release")
            .join("bundle")
            .join(platform_str.to_lowercase());

        let temp_bundle_dir = temp_target_dir
            .join("release")
            .join("bundle")
            .join(platform_str.to_lowercase());

        // Ensure parent directory exists
        if let Some(parent) = final_bundle_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                    command: "create bundle parent directory".to_string(),
                    reason: format!("Failed to create {}: {}", parent.display(), e),
                }))?;
        }

        // Remove old final directory if it exists (safe here because we have artifacts)
        if final_bundle_dir.exists() {
            std::fs::remove_dir_all(&final_bundle_dir)
                .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                    command: "remove old bundle directory".to_string(),
                    reason: format!("Failed to remove {}: {}", final_bundle_dir.display(), e),
                }))?;
        }

        // Atomic rename: only one process can succeed
        std::fs::rename(&temp_bundle_dir, &final_bundle_dir)
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "move artifacts to final location".to_string(),
                reason: format!(
                    "Failed to move {} to {}: {}\n\
                     This may indicate another process completed first.",
                    temp_bundle_dir.display(),
                    final_bundle_dir.display(),
                    e
                ),
            }))?;

        runtime_config.verbose_println(&format!(
            "Moved artifacts from temp to final location: {}",
            final_bundle_dir.display()
        ));

        // Update artifact paths to point to final location
        let artifacts = artifacts
            .into_iter()
            .map(|path| {
                // Replace temp path prefix with final path prefix
                path.strip_prefix(&temp_target_dir)
                    .ok()
                    .map(|rel| self.workspace_path.join("target").join(rel))
                    .unwrap_or(path)
            })
            .collect::<Vec<_>>();

        // Clean up temporary target directory
        std::fs::remove_dir_all(&temp_target_dir)
            .map_err(|e| {
                // Log but don't fail - temp cleanup is not critical
                runtime_config.verbose_println(&format!(
                    "Warning: Failed to clean up temp directory {}: {}",
                    temp_target_dir.display(),
                    e
                ));
            })
            .ok(); // Ignore errors - don't fail build over cleanup

        // Container cleanup strategy:
        // - Normal exit: --rm flag auto-removes container when process exits
        // - Abnormal termination: Guard Drop removes container (SIGKILL, panic, etc.)
        // - Guard running on already-removed container is harmless (errors ignored)

        Ok(artifacts)
    }
}
