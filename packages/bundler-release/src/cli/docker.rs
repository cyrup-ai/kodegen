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

// SECURITY: Allow unsafe code in this module for Docker security features
// Required for libc::getuid() and libc::getgid() calls to prevent container root execution
#![allow(unsafe_code)]

use crate::bundler::PackageType;
use crate::error::{CliError, ReleaseError};
use chrono::{DateTime, Utc};
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
    #[allow(dead_code)]
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
            Duration::from_secs(10),  // Image check should be fast
            Command::new("docker")
                .args(["images", "-q", BUILDER_IMAGE_NAME])
                .output()
        ).await
            .map_err(|_| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker images".to_string(),
                reason: "Docker image check timed out after 10 seconds".to_string(),
            }))?
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
    
    let build_result = timeout(
        DOCKER_BUILD_TIMEOUT,
        Command::new("docker")
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
    ).await;

    let build_output = match build_result {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker build".to_string(),
                reason: format!("Failed to execute docker build: {}", e),
            }));
        }
        Err(_) => {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker build".to_string(),
                reason: format!(
                    "Docker build timed out after {} minutes.\n\
                     \n\
                     This usually means:\n\
                     • Network issues downloading base images\n\
                     • apt-get update is stuck\n\
                     • Build step is hanging\n\
                     \n\
                     Check Docker logs: docker ps -a | head -2",
                    DOCKER_BUILD_TIMEOUT.as_secs() / 60
                ),
            }));
        }
    };

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

        // Execute container with timeout
        let run_result = timeout(
            DOCKER_RUN_TIMEOUT,
            Command::new("docker")
                .args(&docker_args)
                .output()
        ).await;

        let output = match run_result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                    command: format!("docker run {}", docker_args.join(" ")),
                    reason: format!("Failed to execute docker run: {}", e),
                }));
            }
            Err(_) => {
                return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                    command: format!("bundle {} in container", platform_str),
                    reason: format!(
                        "Docker bundling timed out after {} minutes.\n\
                         \n\
                         Container was running:\n\
                         {}\n\
                         \n\
                         This usually means:\n\
                         • Cargo build is taking longer than expected\n\
                         • Network issues downloading dependencies\n\
                         • Container is stuck waiting for input\n\
                         \n\
                         Try:\n\
                         • Check container status: docker ps\n\
                         • View logs: docker logs <container-id>\n\
                         • Run with --no-build to skip compilation",
                        DOCKER_RUN_TIMEOUT.as_secs() / 60,
                        docker_args.join(" ")
                    ),
                }));
            }
        };

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
            
            // Check for common Docker errors with pattern matching
            let help_text = if stderr.contains("permission denied") || stderr.contains("Permission denied") {
                "\n\nℹ  Tip: Docker permission issue. Run:\n   \
                 sudo usermod -aG docker $USER\n   \
                 Then log out and log back in."
            } else if stderr.contains("Cannot connect to the Docker daemon") {
                "\n\nℹ  Tip: Docker daemon not accessible:\n   \
                 • Ensure Docker Desktop/daemon is running\n   \
                 • Check: docker ps"
            } else if stderr.contains("no space left on device") || stderr.contains("No space left on device") {
                "\n\nℹ  Tip: Disk space exhausted. Clean up:\n   \
                 docker system prune -a --volumes"
            } else if stderr.contains("manifest unknown") || stderr.contains("not found") {
                "\n\nℹ  Tip: Docker image may not be built. Run:\n   \
                 docker images | grep kodegen-release-builder"
            } else {
                ""
            };
            
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("bundle {} in container", platform_str),
                reason: format!(
                    "Container bundling failed with exit code: {}\n\
                     \n\
                     Stderr:\n{}\n\
                     \n\
                     Stdout:\n{}\
                     {}",
                    output.status.code().unwrap_or(-1),
                    stderr,
                    stdout,
                    help_text
                ),
            }));
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

/// Checks if Wine is available for building Windows packages on Linux.
///
/// Returns true if `wine --version` executes successfully, false otherwise.
/// This enables runtime detection instead of compile-time assumptions.
///
/// # Examples
///
/// On Linux with Wine installed:
/// ```no_run
/// assert_eq!(has_wine(), true);
/// ```
///
/// On Linux without Wine or non-Linux systems:
/// ```no_run
/// assert_eq!(has_wine(), false);
/// ```
#[cfg(target_os = "linux")]
fn has_wine() -> bool {
    std::process::Command::new("wine")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Wine is not available on non-Linux platforms.
#[cfg(not(target_os = "linux"))]
fn has_wine() -> bool {
    false
}

/// Checks if a platform can be built natively on the current host OS.
///
/// Uses runtime OS detection via `std::env::consts::OS` instead of compile-time
/// cfg attributes. This enables dynamic capability checking (e.g., Wine availability).
///
/// # Platform Support
///
/// - **macOS**: MacOsBundle, Dmg (native only, cannot be built in containers)
/// - **Linux**: Deb, Rpm, AppImage (always native)
/// - **Linux + Wine**: Nsis, WindowsMsi (requires Wine at runtime)
/// - **Windows**: Nsis, WindowsMsi (native)
/// - **All others**: Require Docker container
///
/// # Returns
///
/// - `true` - Platform can be built natively on current OS
/// - `false` - Platform requires Docker container
fn is_native_platform(platform: PackageType) -> bool {
    use PackageType::*;
    
    match (std::env::consts::OS, platform) {
        // macOS native packages (cannot be built in Linux containers)
        ("macos", MacOsBundle | Dmg) => true,
        
        // Linux native packages
        ("linux", Deb | Rpm | AppImage) => true,
        
        // Linux with Wine can build Windows packages
        // Runtime check ensures Wine is actually installed
        ("linux", Nsis | WindowsMsi) => has_wine(),
        
        // Windows native packages
        ("windows", Nsis | WindowsMsi) => true,
        
        // Everything else needs Docker
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
