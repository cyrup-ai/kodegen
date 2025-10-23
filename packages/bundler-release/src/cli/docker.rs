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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Platform-specific Docker startup instructions
#[cfg(target_os = "macos")]
const DOCKER_START_HELP: &str = "Start Docker Desktop from Applications or Spotlight";

#[cfg(target_os = "linux")]
const DOCKER_START_HELP: &str = "Start Docker daemon: sudo systemctl start docker";

#[cfg(target_os = "windows")]
const DOCKER_START_HELP: &str = "Start Docker Desktop from the Start menu";

/// Docker image name for the release builder container
const BUILDER_IMAGE_NAME: &str = "kodegen-release-builder";

/// Docker container bundler for cross-platform builds.
///
/// Manages Docker container lifecycle for building packages on platforms
/// other than the host OS.
#[derive(Debug)]
pub struct ContainerBundler {
    image_name: String,
    workspace_path: PathBuf,
}

impl ContainerBundler {
    /// Creates a new container bundler.
    ///
    /// # Arguments
    ///
    /// * `workspace_path` - Path to the workspace root (will be mounted in container)
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            image_name: BUILDER_IMAGE_NAME.to_string(),
            workspace_path,
        }
    }

    /// Checks if Docker is installed and the daemon is running.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Docker is available
    /// * `Err` - Docker is not installed or daemon is not running
    pub fn check_docker_available() -> Result<(), ReleaseError> {
        let output = Command::new("docker")
            .arg("info")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match output {
            Ok(status) if status.success() => Ok(()),
            
            // Docker command exists but daemon isn't responding
            Ok(status) => {
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
            Err(e) => {
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

    /// Ensures the builder Docker image is built and ready.
    ///
    /// Checks if the image exists, and builds it if not.
    ///
    /// # Arguments
    ///
    /// * `workspace_path` - Path to workspace containing .devcontainer/Dockerfile
    /// * `runtime_config` - Runtime configuration for output
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Image is ready
    /// * `Err` - Failed to build image
    pub fn ensure_image_built(workspace_path: &Path, runtime_config: &crate::cli::RuntimeConfig) -> Result<(), ReleaseError> {
        // Check if image exists
        let check_output = Command::new("docker")
            .args(["images", "-q", BUILDER_IMAGE_NAME])
            .output()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker images".to_string(),
                reason: e.to_string(),
            }))?;

        if !check_output.stdout.is_empty() {
            // Image exists
            return Ok(());
        }

        // Image doesn't exist, build it
        runtime_config.progress(&format!("Building {} Docker image (this may take a few minutes)...", BUILDER_IMAGE_NAME));
        
        let dockerfile_path = workspace_path.join(".devcontainer");
        
        let build_output = Command::new("docker")
            .args([
                "build",
                "-t",
                BUILDER_IMAGE_NAME,
                "-f",
                "Dockerfile",
                "."
            ])
            .current_dir(&dockerfile_path)
            .output()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "docker build".to_string(),
                reason: e.to_string(),
            }))?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            let stdout = String::from_utf8_lossy(&build_output.stdout);
            
            // Parse stderr for common error patterns and provide specific help
            let help_text = if stderr.contains("permission denied") || stderr.contains("Permission denied") {
                "\n\nℹ  Tip: Add your user to the docker group:\n   \
                 sudo usermod -aG docker $USER\n   \
                 Then log out and back in for the change to take effect.\n   \
                 Or run with sudo (not recommended for regular use)."
            } else if stderr.contains("Cannot connect to the Docker daemon") || stderr.contains("Is the docker daemon running") {
                "\n\nℹ  Tip: Ensure Docker daemon is running:\n   \
                 • macOS/Windows: Start Docker Desktop\n   \
                 • Linux: sudo systemctl start docker"
            } else if stderr.contains("no space left on device") || stderr.contains("No space left on device") {
                "\n\nℹ  Tip: Clean up Docker resources:\n   \
                 docker system prune -a --volumes\n   \
                 This will remove unused images, containers, and volumes."
            } else if stderr.contains("Dockerfile not found") || stderr.contains("Cannot locate specified Dockerfile") {
                "\n\nℹ  Tip: Ensure .devcontainer/Dockerfile exists in workspace root:\n   \
                 Expected path: .devcontainer/Dockerfile"
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

        // Build docker run command with owned strings for clear ownership
        let mount_arg = format!("{}:/workspace", self.workspace_path.display());
        
        let mut docker_args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-v".to_string(),
            mount_arg,
            "-w".to_string(),
            "/workspace".to_string(),
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

        // Execute container
        let output = Command::new("docker")
            .args(&docker_args)
            .output()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("docker run {}", docker_args.join(" ")),
                reason: e.to_string(),
            }))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Parse for actionable error patterns
            let help_text = if stderr.contains("permission denied") || stderr.contains("Permission denied") {
                "\n\nℹ  Tip: Docker permission issue. Run:\n   \
                 sudo usermod -aG docker $USER\n   \
                 Then log out and log back in."
            } else if stderr.contains("Cannot connect to the Docker daemon") {
                "\n\nℹ  Tip: Docker daemon not accessible:\n   \
                 • Ensure Docker Desktop/daemon is running\n   \
                 • Check: docker ps"
            } else if stderr.contains("no space left on device") {
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
                    "Container bundling failed:\n\
                     \n\
                     Stderr:\n{}\n\
                     \n\
                     Stdout:\n{}\
                     {}",
                    stderr, stdout, help_text
                ),
            }));
        }

        runtime_config.indent(&format!("✓ Created {} package", platform_str));

        // Find created artifacts using case-insensitive directory search
        let bundle_dir = find_bundle_directory(&self.workspace_path, &platform_str)?;

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
fn platform_type_to_string(platform: PackageType) -> String {
    match platform {
        PackageType::Deb => "deb",
        PackageType::Rpm => "rpm",
        PackageType::AppImage => "appimage",
        PackageType::MacOsBundle => "app",
        PackageType::Dmg => "dmg",
        PackageType::WindowsMsi => "msi",
        PackageType::Nsis => "nsis",
    }.to_string()
}

/// Returns emoji for platform type (for pretty output).
fn platform_emoji(platform: PackageType) -> &'static str {
    match platform {
        PackageType::Deb | PackageType::Rpm | PackageType::AppImage => "🐧",
        PackageType::MacOsBundle | PackageType::Dmg => "🍎",
        PackageType::WindowsMsi | PackageType::Nsis => "🪟",
    }
}
