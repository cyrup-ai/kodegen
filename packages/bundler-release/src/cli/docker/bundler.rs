//! Docker container bundler for cross-platform builds.
//!
//! Manages Docker container lifecycle for building packages on platforms
//! other than the host OS.

// SECURITY: Allow unsafe code in this module for Docker security features
// Required for libc::getuid() and libc::getgid() calls to prevent container root execution
#![allow(unsafe_code)]

use super::artifacts::{find_bundle_directory, verify_artifacts};
use super::guard::ContainerGuard;
use super::limits::ContainerLimits;
use super::platform::{platform_emoji, platform_type_to_string};
use crate::bundler::PackageType;
use crate::error::{CliError, ReleaseError};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
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

        // Clean old artifacts BEFORE running container to prevent stale/corrupted files
        let bundle_dir = self.workspace_path
            .join("target")
            .join("release")
            .join("bundle")
            .join(platform_str.to_lowercase());

        if bundle_dir.exists() {
            runtime_config.indent(&format!("  Cleaning old {} artifacts...", platform_str));
            std::fs::remove_dir_all(&bundle_dir)
                .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                    command: "clean old artifacts".to_string(),
                    reason: format!("Failed to remove {}: {}", bundle_dir.display(), e),
                }))?;
        }

        // Generate unique container name for tracking and cleanup
        let container_name = format!("kodegen-bundle-{}", Uuid::new_v4());

        // Create RAII guard to ensure cleanup on failure
        // Guard will automatically call `docker rm -f` when dropped (on error or panic)
        let _guard = ContainerGuard {
            name: container_name.clone(),
        };

        // SECURITY: Validate and canonicalize workspace path to resolve symlinks
        let workspace_path = self.workspace_path
            .canonicalize()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "validate workspace path".to_string(),
                reason: format!(
                    "Invalid workspace path '{}': {}\n\
                     \n\
                     Possible causes:\n\
                     • Path does not exist\n\
                     • Path contains invalid symlinks\n\
                     • Insufficient permissions to access path\n\
                     \n\
                     Ensure the workspace path exists and is accessible.",
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

        // SECURITY: Verify target directory exists or can be created
        let target_dir = workspace_path.join("target");
        if !target_dir.exists() {
            std::fs::create_dir_all(&target_dir)
                .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                    command: "create target directory".to_string(),
                    reason: format!(
                        "Failed to create target directory: {}\n\
                         This directory is required for build outputs.",
                        e
                    ),
                }))?;
        }

        // SECURITY: Get current user ID to map into container (prevents root execution)
        // This ensures files created in container have correct ownership
        #[cfg(unix)]
        let user_mapping = {
            let uid = unsafe { libc::getuid() };
            let gid = unsafe { libc::getgid() };
            format!("{}:{}", uid, gid)
        };

        #[cfg(not(unix))]
        let user_mapping = {
            // Windows containers run with different security model
            // Use default container user (builder from Dockerfile)
            String::new()
        };

        // SECURITY: Build secure mount arguments
        // Mount workspace as read-only (prevents source code modification)
        let workspace_mount = format!("{}:/workspace:ro", workspace_path.display());

        // Mount target/ as read-write (required for build outputs)
        let target_mount = format!("{}:/workspace/target:rw", target_dir.display());

        // Build docker arguments with security constraints
        let mut docker_args = vec![
            "run".to_string(),
            "--name".to_string(),
            container_name.clone(),
            
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

        // SECURITY: Add user mapping on Unix systems (prevents running as root)
        #[cfg(unix)]
        if !user_mapping.is_empty() {
            docker_args.push("--user".to_string());
            docker_args.push(user_mapping);
        }

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

        // Run docker with captured output for OOM detection
        let output = tokio::time::timeout(
            DOCKER_RUN_TIMEOUT,
            Command::new("docker")
                .args(&docker_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        ).await
            .map_err(|_| ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("bundle {} in container", platform_str),
                reason: format!(
                    "Docker bundling timed out after {} minutes",
                    DOCKER_RUN_TIMEOUT.as_secs() / 60
                ),
            }))?
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("docker run {}", docker_args.join(" ")),
                reason: e.to_string(),
            }))?;

        // Display stdout
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        for line in stdout_str.lines() {
            runtime_config.indent(line);
        }

        if !output.status.success() {
            // Check for OOM indicators in stderr
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            let is_oom = stderr_str.contains("OOMKilled") 
                || stderr_str.contains("out of memory")
                || stderr_str.contains("OutOfMemoryError");

            if is_oom {
                // Get system memory info
                let mut sys = sysinfo::System::new_all();
                sys.refresh_memory();
                let total_memory_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

                return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                    command: format!("bundle {} in container", platform_str),
                    reason: format!(
                        "Container ran out of memory during build.\n\
                         \n\
                         Current memory limit: {} (swap: {})\n\
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
                    ),
                }));
            } else {
                // Generic failure with captured output
                let error_output = if !stderr_str.is_empty() {
                    format!("stderr:\n{}\n\nstdout:\n{}", stderr_str, stdout_str)
                } else {
                    format!("stdout:\n{}", stdout_str)
                };

                return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                    command: format!("bundle {} in container", platform_str),
                    reason: format!(
                        "Container bundling failed with exit code: {}\n\n{}",
                        output.status.code().unwrap_or(-1),
                        error_output
                    ),
                }));
            }
        }

        runtime_config.indent(&format!("✓ Created {} package", platform_str));

        // Find created artifacts using case-insensitive directory search
        let bundle_dir = find_bundle_directory(&self.workspace_path, platform_str)?;

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
            runtime_config.verbose_println(&format!("  Found: {}", path.display()));
            
            if path.is_file() {
                artifacts.push(path);
            } else if path.is_dir() {
                runtime_config.verbose_println(&format!("  Skipping directory: {}", path.display()));
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

        // Success! Disarm the guard to skip cleanup (container will auto-cleanup via Docker)
        // We remove guard responsibility because container succeeded and Docker will clean it up
        std::mem::forget(_guard);

        Ok(artifacts)
    }
}
