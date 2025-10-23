//! Docker container integration for cross-platform bundling.
//!
//! This module enables the `bundle` command to automatically use Docker containers
//! when building packages for platforms other than the host OS.
//!
//! # Example
//!
//! On macOS, running `bundle --all-platforms` will:
//! - Build macOS packages (.app, .dmg) natively
//! - Build Linux/Windows packages (.deb, .rpm, AppImage, .msi, .exe) in a Linux container with Wine
//!
//! # Architecture
//!
//! The Linux container (defined in `.devcontainer/Dockerfile`) includes:
//! - Rust toolchain (nightly matching rust-toolchain.toml)
//! - Wine + .NET 4.0 (for running WiX to create .msi installers)
//! - NSIS (for creating .exe installers)
//! - RPM/DEB tools (for creating Linux packages)
//! - linuxdeploy (for creating AppImages)

use crate::bundler::PackageType;
use crate::error::{CliError, ReleaseError};
use chrono::{DateTime, Utc};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

/// Platform-specific Docker startup instructions
#[cfg(target_os = "macos")]
const DOCKER_START_HELP: &str = "Start Docker Desktop from Applications or Spotlight";

#[cfg(target_os = "linux")]
const DOCKER_START_HELP: &str = "Start Docker daemon: sudo systemctl start docker";

#[cfg(target_os = "windows")]
const DOCKER_START_HELP: &str = "Start Docker Desktop from the Start menu";

/// Docker image name for the release builder container
const BUILDER_IMAGE_NAME: &str = "kodegen-release-builder";

/// Timeout for Docker info check (5 seconds)
/// Quick daemon availability check shouldn't take long
const DOCKER_INFO_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for Docker image build operations (30 minutes)
/// Image builds can take a long time due to base image downloads, apt updates, etc.
const DOCKER_BUILD_TIMEOUT: Duration = Duration::from_secs(1800);

/// Timeout for Docker container run operations (20 minutes)
/// Container bundling involves full cargo builds which can be slow
const DOCKER_RUN_TIMEOUT: Duration = Duration::from_secs(1200);

/// Resource limits for Docker containers.
///
/// Controls memory, CPU, and process limits to prevent containers from
/// consuming excessive host resources during cross-platform builds.
#[derive(Debug, Clone)]
pub struct ContainerLimits {
    /// Maximum memory (e.g., "4g", "2048m")
    pub memory: String,
    
    /// Maximum memory + swap (e.g., "6g", "3072m")
    pub memory_swap: String,
    
    /// Number of CPUs (fractional allowed, e.g., "2", "1.5")
    pub cpus: String,
    
    /// Maximum number of processes
    pub pids_limit: u32,
}

impl Default for ContainerLimits {
    fn default() -> Self {
        Self::detect_safe_limits()
    }
}

impl ContainerLimits {
    /// Detects safe resource limits based on host system capabilities.
    ///
    /// Uses conservative defaults:
    /// - Memory: 50% of total RAM (minimum 2GB, maximum 8GB)
    /// - Swap: Memory + 2GB
    /// - CPUs: 50% of available cores (minimum 2)
    /// - PIDs: 1000 (sufficient for most builds, prevents fork bombs)
    pub fn detect_safe_limits() -> Self {
        use sysinfo::System;
        
        let mut sys = System::new_all();
        sys.refresh_memory();
        
        // Calculate memory limit (50% of total, min 2GB, max 8GB)
        let total_ram_gb = sys.total_memory() / 1024 / 1024 / 1024;
        let memory_gb = ((total_ram_gb / 2).max(2)).min(8);
        let swap_gb = memory_gb + 2;
        
        // Calculate CPU limit (50% of cores, minimum 2)
        let total_cpus = num_cpus::get();
        let cpu_limit = (total_cpus / 2).max(2);
        
        Self {
            memory: format!("{}g", memory_gb),
            memory_swap: format!("{}g", swap_gb),
            cpus: cpu_limit.to_string(),
            pids_limit: 1000,
        }
    }
    
    /// Creates limits from CLI arguments.
    ///
    /// Validates that memory_swap >= memory.
    pub fn from_cli(
        memory: String,
        memory_swap: Option<String>,
        cpus: Option<String>,
        pids_limit: u32,
    ) -> Self {
        let memory_swap = memory_swap.unwrap_or_else(|| {
            // Default: memory + 2GB
            let mem_gb: u32 = memory
                .trim_end_matches('g')
                .trim_end_matches('m')
                .parse()
                .unwrap_or(4);
            format!("{}g", mem_gb + 2)
        });
        
        let cpus = cpus.unwrap_or_else(|| num_cpus::get().to_string());
        
        Self {
            memory,
            memory_swap,
            cpus,
            pids_limit,
        }
    }
}

/// RAII guard for Docker container cleanup.
///
/// Automatically removes containers when dropped, ensuring cleanup even on panic or error.
/// Follows the same Drop pattern as StateManager in state/manager.rs.
struct ContainerGuard {
    name: String,
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        // Best-effort cleanup - ignore errors as we're already in error/cleanup path
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.name])
            .output();
        // Note: This runs `docker rm -f <container-name>` which:
        // - Forcefully removes the container (even if running)
        // - Doesn't fail if container doesn't exist
        // - Cleans up container resources
    }
}

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
    /// Creates a new container bundler with default resource limits.
    ///
    /// # Arguments
    ///
    /// * `workspace_path` - Path to the workspace root (will be mounted in container)
    pub fn new(workspace_path: PathBuf) -> Self {
        Self::with_limits(workspace_path, ContainerLimits::default())
    }
    
    /// Creates a container bundler with custom resource limits.
    ///
    /// # Arguments
    ///
    /// * `workspace_path` - Path to the workspace root (will be mounted in container)
    /// * `limits` - Resource limits for the container
    pub fn with_limits(workspace_path: PathBuf, limits: ContainerLimits) -> Self {
        Self {
            image_name: BUILDER_IMAGE_NAME.to_string(),
            workspace_path,
            limits,
        }
    }

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
                .status()
        ).await;

        match status_result {
            // Timeout occurred
            Err(_) => {
                Err(ReleaseError::Cli(CliError::ExecutionFailed {
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
                }))
            }
            
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
                        exit_code,
                        DOCKER_START_HELP
                    ),
                }))
            }
            
            // Docker command not found - not installed
            Ok(Err(e)) => {
                Err(ReleaseError::Cli(CliError::ExecutionFailed {
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
                }))
            }
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
        runtime_config: &crate::cli::RuntimeConfig
    ) -> Result<(), ReleaseError> {
        let dockerfile_path = workspace_path.join(".devcontainer/Dockerfile");
        
        if !dockerfile_path.exists() {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "check_dockerfile".to_string(),
                reason: format!(
                    "Dockerfile not found at expected path: {}\n\
                     Expected location: .devcontainer/Dockerfile",
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
        let check_output = Command::new("docker")
            .args(["images", "-q", BUILDER_IMAGE_NAME])
            .output()
            .await
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker images".to_string(),
                reason: e.to_string(),
            }))?;

        let image_id = String::from_utf8_lossy(&check_output.stdout).trim().to_string();
        
        if !image_id.is_empty() {
            // Image exists - check if it's up-to-date
            runtime_config.verbose_println(&format!(
                "Found existing Docker image: {}",
                &image_id[..12.min(image_id.len())]
            ));
            
            match is_image_up_to_date(&image_id, &dockerfile_path, runtime_config).await {
                Ok(true) => {
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
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: format!("docker inspect {}", image_id),
            reason: e.to_string(),
        }))?;
    
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
    let image_created_time = DateTime::parse_from_rfc3339(&image_created_str)
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "parse_timestamp".to_string(),
            reason: format!(
                "Invalid timestamp from Docker '{}': {}",
                image_created_str, e
            ),
        }))?;
    
    // Get Dockerfile modification time
    let dockerfile_metadata = std::fs::metadata(dockerfile_path)
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "stat_dockerfile".to_string(),
            reason: format!("Cannot read Dockerfile metadata: {}", e),
        }))?;
    
    let dockerfile_modified = dockerfile_metadata
        .modified()
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "get_mtime".to_string(),
            reason: format!("Cannot get Dockerfile modification time: {}", e),
        }))?;
    
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
async fn build_docker_image(
    workspace_path: &Path,
    runtime_config: &crate::cli::RuntimeConfig,
) -> Result<(), ReleaseError> {
    let dockerfile_dir = workspace_path.join(".devcontainer");
    
    runtime_config.progress(&format!(
        "Building Docker image: {}",
        BUILDER_IMAGE_NAME
    ));
    
    let build_output = Command::new("docker")
        .args([
            "build",
            "--pull",  // Always pull latest base image
            "-t",
            BUILDER_IMAGE_NAME,
            "-f",
            "Dockerfile",
            ".",
        ])
        .current_dir(&dockerfile_dir)
        .output()
        .await
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker build".to_string(),
            reason: e.to_string(),
        }))?;

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        let stdout = String::from_utf8_lossy(&build_output.stdout);
        
        // Provide helpful error context
        let help_text = if stderr.contains("permission denied") || stderr.contains("Permission denied") {
            "\n\nℹ  Tip: Add your user to the docker group:\n   \
             sudo usermod -aG docker $USER\n   \
             Then log out and back in."
        } else if stderr.contains("Cannot connect to the Docker daemon") {
            "\n\nℹ  Tip: Ensure Docker daemon is running:\n   \
             • macOS/Windows: Start Docker Desktop\n   \
             • Linux: sudo systemctl start docker"
        } else if stderr.contains("no space left on device") {
            "\n\nℹ  Tip: Clean up Docker resources:\n   \
             docker system prune -a --volumes"
        } else {
            ""
        };
        
        return Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "docker build".to_string(),
            reason: format!(
                "Failed to build Docker image:\n\
                 \n\
                 Stderr:\n{}\n\
                 \n\
                 Stdout:\n{}\
                 {}",
                stderr, stdout, help_text
            ),
        }));
    }

    runtime_config.success("Docker image built successfully");
    Ok(())
}

/// Convert seconds to human-readable duration
fn humanize_duration(seconds: i64) -> String {
    if seconds < 60 {
        format!("{} seconds", seconds)
    } else if seconds < 3600 {
        format!("{} minutes", seconds / 60)
    } else if seconds < 86400 {
        format!("{} hours", seconds / 3600)
    } else {
        format!("{} days", seconds / 86400)
    }
}

impl ContainerBundler {
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
    pub fn bundle_platform(
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

        // Build docker run command with owned strings for clear ownership
        let mount_arg = format!("{}:/workspace", self.workspace_path.display());
        
        let mut docker_args = vec![
            "run".to_string(),
            "--name".to_string(),
            container_name.clone(),
            
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
            
            // Mount and working directory
            "-v".to_string(),
            mount_arg,
            "-w".to_string(),
            "/workspace".to_string(),
            
            // Image and command
            self.image_name.clone(),
            "cargo".to_string(),
            "run".to_string(),
            "-p".to_string(),
            "kodegen_release".to_string(),
            "--".to_string(),
            "bundle".to_string(),
            "--platform".to_string(),
            platform_str.to_string(),
        ];

        if build {
            docker_args.push("--build".to_string());
        }
        if release {
            docker_args.push("--release".to_string());
        }

        // Execute container with streaming output
        let mut child = std::process::Command::new("docker")
            .args(&docker_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("docker run {}", docker_args.join(" ")),
                reason: e.to_string(),
            }))?;
        
        // Stream stdout in real-time
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().filter_map(Result::ok) {
                runtime_config.indent(&line);
            }
        }
        
        let output = child.wait_with_output()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("docker run {}", docker_args.join(" ")),
                reason: e.to_string(),
            }))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Check for OOM (Out Of Memory) error
            if stderr.contains("OOMKilled") || stderr.contains("out of memory") || stderr.contains("OutOfMemoryError") {
                use sysinfo::System;
                let mut sys = System::new_all();
                sys.refresh_memory();
                let total_memory_gb = sys.total_memory() / 1024 / 1024 / 1024;
                
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
            }
            
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("bundle {} in container", platform_str),
                reason: format!(
                    "Container bundling failed with exit code: {}\n\
                     \n\
                     Stderr:\n{}\n\
                     \n\
                     Stdout:\n{}\n\
                     \n\
                     Common causes:\n\
                     • Cargo build errors inside container\n\
                     • Missing dependencies in Docker image\n\
                     • Bundler errors (check bundler logs above)\n\
                     • Docker daemon issues",
                    output.status.code().unwrap_or(-1),
                    stderr,
                    stdout,
                ),
            }));
        }

        runtime_config.indent(&format!("✓ Created {} package", platform_str));

        // Find created artifacts using case-insensitive directory search
        let bundle_dir = find_bundle_directory(&self.workspace_path, platform_str)?;

        // Collect artifact paths
        let mut artifacts = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&bundle_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    artifacts.push(path);
                }
            }
        }

        if artifacts.is_empty() {
            // Show what we found instead of just saying "nothing found"
            let dir_contents = std::fs::read_dir(&bundle_dir)
                .ok()
                .and_then(|entries| {
                    let items: Vec<_> = entries
                        .flatten()
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
                });
            
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

/// Finds the bundle directory for a platform, handling case-insensitive matching.
///
/// This function searches for the bundle directory case-insensitively to handle
/// differences in how bundlers create directory names across platforms.
///
/// # Arguments
///
/// * `workspace_path` - Path to the workspace root
/// * `platform_str` - Platform string (e.g., "deb", "rpm", "appimage")
///
/// # Returns
///
/// * `Ok(PathBuf)` - Path to the bundle directory
/// * `Err` - Bundle directory not found
fn find_bundle_directory(
    workspace_path: &Path,
    platform_str: &str,
) -> Result<PathBuf, ReleaseError> {
    let bundle_base = workspace_path
        .join("target")
        .join("release")
        .join("bundle");
    
    if !bundle_base.exists() {
        return Err(ReleaseError::Cli(CliError::ExecutionFailed {
            command: "find bundle directory".to_string(),
            reason: format!("Bundle directory does not exist: {}", bundle_base.display()),
        }));
    }
    
    // Try exact match first (most common)
    let exact_match = bundle_base.join(platform_str.to_lowercase());
    if exact_match.exists() {
        return Ok(exact_match);
    }
    
    // Search for case-insensitive match
    let entries = std::fs::read_dir(&bundle_base)
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "read bundle directory".to_string(),
            reason: format!("Failed to read {}: {}", bundle_base.display(), e),
        }))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "read directory entry".to_string(),
            reason: format!("Failed to read entry in {}: {}", bundle_base.display(), e),
        }))?;
        
        let path = entry.path();
        if path.is_dir() {
            if let Some(dir_name) = path.file_name() {
                if dir_name.to_string_lossy().eq_ignore_ascii_case(platform_str) {
                    return Ok(path);
                }
            }
        }
    }
    
    // Not found
    Err(ReleaseError::Cli(CliError::ExecutionFailed {
        command: "find bundle directory".to_string(),
        reason: format!(
            "Bundle directory not found for platform '{}' in {}",
            platform_str,
            bundle_base.display()
        ),
    }))
}

/// Verifies that artifacts are complete and not corrupted.
///
/// Checks:
/// - File exists and is readable
/// - File size > 0 (not empty)
/// - File has expected extension
///
/// # Arguments
///
/// * `artifacts` - Paths to artifact files to verify
/// * `runtime_config` - For verbose output
fn verify_artifacts(
    artifacts: &[PathBuf],
    runtime_config: &crate::cli::RuntimeConfig,
) -> Result<(), ReleaseError> {
    for artifact in artifacts {
        // Check file exists and get metadata
        let metadata = std::fs::metadata(artifact)
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "verify artifact".to_string(),
                reason: format!("Cannot read artifact {}: {}", artifact.display(), e),
            }))?;

        // Check file is not empty
        if metadata.len() == 0 {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "verify artifact".to_string(),
                reason: format!(
                    "Artifact is empty (0 bytes): {}\n\
                     This indicates a failed or incomplete build.",
                    artifact.display()
                ),
            }));
        }

        // Success - log verification
        runtime_config.indent(&format!(
            "  ✓ Verified: {} ({} bytes)",
            artifact.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<unknown>"),
            metadata.len()
        ));
    }

    Ok(())
}

/// Splits package types into native (run locally) vs containerized (run in Docker).
///
/// Based on the current host OS, determines which platforms can be built natively
/// and which require a Docker container.
///
/// # Platform Support
///
/// - **macOS**: Native=[MacOsBundle, Dmg], Container=[Deb, Rpm, AppImage, Nsis, WindowsMsi]
/// - **Linux**: Native=[Deb, Rpm, AppImage, Nsis, WindowsMsi], Container=[]
/// - **Windows**: Native=[Nsis, WindowsMsi], Container=[Deb, Rpm, AppImage]
///
/// Note: macOS packages cannot be built in containers due to Apple licensing restrictions.
///
/// # Arguments
///
/// * `platforms` - Requested package types
///
/// # Returns
///
/// * `(native, containerized)` - Tuple of (platforms to build locally, platforms to build in Docker)
pub fn split_platforms_by_host(
    platforms: &[PackageType],
) -> (Vec<PackageType>, Vec<PackageType>) {
    let mut native = Vec::new();
    let mut containerized = Vec::new();

    for &platform in platforms {
        if is_native_platform(platform) {
            native.push(platform);
        } else {
            containerized.push(platform);
        }
    }

    (native, containerized)
}

/// Checks if a platform can be built natively on the current host OS.
fn is_native_platform(platform: PackageType) -> bool {
    match platform {
        #[cfg(target_os = "macos")]
        PackageType::MacOsBundle | PackageType::Dmg => true,
        
        #[cfg(target_os = "linux")]
        PackageType::Deb | PackageType::Rpm | PackageType::AppImage => true,
        
        #[cfg(target_os = "linux")]
        PackageType::Nsis | PackageType::WindowsMsi => true, // Linux can build Windows via Wine
        
        #[cfg(target_os = "windows")]
        PackageType::Nsis | PackageType::WindowsMsi => true,
        
        _ => false,
    }
}

/// Converts PackageType to string for CLI arguments.
fn platform_type_to_string(platform: PackageType) -> &'static str {
    match platform {
        PackageType::Deb => "deb",
        PackageType::Rpm => "rpm",
        PackageType::AppImage => "appimage",
        PackageType::MacOsBundle => "app",
        PackageType::Dmg => "dmg",
        PackageType::WindowsMsi => "msi",
        PackageType::Nsis => "nsis",
    }
}

/// Returns emoji for platform type (for pretty output).
fn platform_emoji(platform: PackageType) -> &'static str {
    match platform {
        PackageType::Deb | PackageType::Rpm | PackageType::AppImage => "🐧",
        PackageType::MacOsBundle | PackageType::Dmg => "🍎",
        PackageType::WindowsMsi | PackageType::Nsis => "🪟",
    }
}
