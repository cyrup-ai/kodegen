//! Bundle helper functions.
//!
//! Shared utilities for bundle creation and GitHub upload.

use crate::cli::RuntimeConfig;
use crate::error::{Result, ReleaseError, CliError};
use crate::git::GitManager;

/// Default product name for bundles
const DEFAULT_PRODUCT_NAME: &str = "kodegen";

/// Default product description for bundles
const DEFAULT_PRODUCT_DESCRIPTION: &str =
    "KODEGEN.ᴀɪ: Memory-efficient, Blazing-Fast, MCP tools for code generation agents.";

/// Discover binaries from workspace members by parsing Cargo.toml
pub(super) fn discover_binaries_from_workspace(
    workspace: &crate::workspace::WorkspaceInfo,
) -> Result<Vec<crate::bundler::BundleBinary>> {
    let mut binaries = Vec::new();

    for member in workspace.packages.values() {
        if has_binary_target(member)? {
            let name = member.name.clone();

            // kodegen_install is the main executable (installer launcher)
            // kodegen and kodegend are resources to be installed
            let is_main = name == "kodegen_install";

            binaries.push(crate::bundler::BundleBinary::new(name, is_main));
        }
    }

    // Verify kodegen_install was found and marked as main
    if !binaries.iter().any(|b| b.main()) {
        return Err(ReleaseError::Cli(CliError::InvalidArguments {
            reason: "kodegen_install binary not found in workspace. Required for bundling.".to_string(),
        }));
    }

    Ok(binaries)
}

/// Check if package has binary targets by parsing Cargo.toml
fn has_binary_target(package: &crate::workspace::PackageInfo) -> Result<bool> {
    let manifest_path = &package.cargo_toml_path;
    let manifest_content = std::fs::read_to_string(manifest_path)
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "read_manifest".to_string(),
            reason: e.to_string(),
        }))?;

    // Simple heuristic: look for [[bin]] section or bin flag
    Ok(manifest_content.contains("[[bin]]") ||
       (manifest_content.contains("name = ") && !manifest_content.contains("[lib]")))
}

/// Build workspace binaries with cargo
pub(crate) fn build_workspace_binaries(workspace_path: &std::path::Path, release: bool) -> Result<()> {
    use std::process::Command;

    eprintln!("   Working directory: {}", workspace_path.display());
    
    // Required binaries for kodegen release
    let required_binaries = ["kodegen_install", "kodegen", "kodegend"];
    
    for binary in &required_binaries {
        eprintln!("   Building binary: {}{}", binary, if release { " (release mode)" } else { "" });
        
        let mut cmd = Command::new("cargo");
        cmd.current_dir(workspace_path);
        cmd.arg("build");
        cmd.arg("--bin");
        cmd.arg(binary);

        if release {
            cmd.arg("--release");
        }

        let output = cmd.output()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("cargo_build_{}", binary),
                reason: e.to_string(),
            }))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: format!("cargo_build_{}", binary),
                reason: format!("Failed to build {}:\n{}", binary, stderr),
            }));
        }
        
        eprintln!("   ✓ {} built successfully", binary);
    }

    Ok(())
}

/// Create bundler settings from workspace analysis
pub(super) fn create_bundler_settings(
    workspace: &crate::workspace::WorkspaceInfo,
    binaries: &[crate::bundler::BundleBinary],
    override_name: &Option<String>,
    override_version: &Option<String>,
    release: bool,
    target_override: &Option<String>,
    platform: Option<&str>,
) -> Result<crate::bundler::Settings> {
    use crate::bundler::{SettingsBuilder, PackageSettings};

    // Use product-level metadata (deterministic and semantically correct)
    let name = DEFAULT_PRODUCT_NAME.to_string();
    let description = DEFAULT_PRODUCT_DESCRIPTION.to_string();

    // Get version from workspace configuration
    let version = workspace
        .workspace_config
        .package
        .as_ref()
        .and_then(|p| p.version.clone())
        .unwrap_or_else(|| "0.0.0".to_string());

    // Determine output directory
    let out_dir = if release {
        std::path::PathBuf::from("target/release")
    } else {
        std::path::PathBuf::from("target/debug")
    };

    // Determine target triple
    let target = target_override.clone().unwrap_or_else(|| {
        std::env::var("TARGET").unwrap_or_else(|_| {
            // Detect from current platform
            if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
                "x86_64-unknown-linux-gnu"
            } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "macos") {
                "x86_64-apple-darwin"
            } else if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
                "aarch64-apple-darwin"
            } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
                "x86_64-pc-windows-msvc"
            } else {
                "unknown"
            }.to_string()
        })
    });

    // Build settings
    let mut builder = SettingsBuilder::new()
        .project_out_directory(&out_dir)
        .package_settings(PackageSettings {
            product_name: override_name.clone().unwrap_or(name.clone()),
            version: override_version.clone().unwrap_or(version),
            description,
            homepage: None,
            authors: None,
            default_run: None,
        })
        .binaries(binaries.to_vec())
        .target(target);

    // Set package types if specified
    if let Some(platform_str) = platform {
        let package_type = parse_package_type(platform_str)?;
        builder = builder.package_types(vec![package_type]);
    }

    // Configure platform-specific settings
    use std::path::PathBuf;
    use crate::bundler::{BundleSettings, DebianSettings, RpmSettings, MacOsSettings, WindowsSettings};

    // Read signing configuration from environment variables
    let macos_settings = MacOsSettings {
        signing_identity: std::env::var("MACOS_SIGNING_IDENTITY").ok(),
        entitlements: std::env::var("MACOS_ENTITLEMENTS_PATH").ok().map(PathBuf::from),
        skip_notarization: std::env::var("MACOS_SKIP_NOTARIZATION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        ..Default::default()
    };

    let windows_settings = WindowsSettings {
        cert_path: std::env::var("WINDOWS_CERT_PATH").ok().map(PathBuf::from),
        key_path: std::env::var("WINDOWS_KEY_PATH").ok().map(PathBuf::from),
        password: std::env::var("WINDOWS_CERT_PASSWORD").ok(),
        timestamp_url: std::env::var("WINDOWS_TIMESTAMP_URL").ok(),
        ..Default::default()
    };

    // Configure icon paths from assets directory (multi-resolution)
    let icon_paths = vec![
        workspace.root.join("assets/img/icon_16x16.png"),
        workspace.root.join("assets/img/icon_16x16@2x.png"),
        workspace.root.join("assets/img/icon_32x32.png"),
        workspace.root.join("assets/img/icon_32x32@2x.png"),
        workspace.root.join("assets/img/icon_128x128.png"),
        workspace.root.join("assets/img/icon_128x128@2x.png"),
        workspace.root.join("assets/img/icon_256x256.png"),
        workspace.root.join("assets/img/icon_256x256@2x.png"),
        workspace.root.join("assets/img/icon_512x512.png"),
        workspace.root.join("assets/img/icon_512x512@2x.png"),
    ];

    let bundle_settings = BundleSettings {
        identifier: Some(format!("ai.kodegen.{}", override_name.clone().unwrap_or_else(|| name.clone()))),
        icon: Some(icon_paths),
        deb: DebianSettings {
            post_install_script: Some(PathBuf::from("packages/bundler-release/postinst.deb.sh")),
            ..Default::default()
        },
        rpm: RpmSettings {
            post_install_script: Some(PathBuf::from("packages/bundler-release/postinst.rpm.sh")),
            ..Default::default()
        },
        macos: macos_settings,
        windows: windows_settings,
        ..Default::default()
    };

    builder = builder.bundle_settings(bundle_settings);

    // Validate that all binaries exist before bundling
    let binary_dir = workspace.root.join(&out_dir);
    for binary in binaries {
        let binary_path = binary_dir.join(binary.name());
        let binary_path_exe = binary_dir.join(format!("{}.exe", binary.name()));

        if !binary_path.exists() && !binary_path_exe.exists() {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "validate_binaries".to_string(),
                reason: format!(
                    "Binary '{}' not found in {}\n\
                     Build all binaries first:\n  \
                     cargo build --release --bin kodegen_install\n  \
                     cargo build --release --bin kodegen\n  \
                     cargo build --release --bin kodegend",
                    binary.name(),
                    binary_dir.display()
                ),
            }));
        }
    }

    builder.build()
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "build_settings".to_string(),
            reason: e.to_string(),
        }))
}

/// Calculate SHA-256 checksum of a file
pub(super) fn calculate_artifact_checksum(path: &std::path::Path) -> Result<String> {
    use sha2::{Sha256, Digest};
    use std::io::Read;

    let mut file = std::fs::File::open(path)
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "read_file_for_checksum".to_string(),
            reason: e.to_string(),
        }))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let n = file.read(&mut buffer)
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "read_file_for_checksum".to_string(),
                reason: e.to_string(),
            }))?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Parse package type string to enum
pub(super) fn parse_package_type(platform_str: &str) -> Result<crate::bundler::PackageType> {
    use crate::bundler::PackageType;

    match platform_str.to_lowercase().as_str() {
        "deb" | "debian" => Ok(PackageType::Deb),
        "rpm" => Ok(PackageType::Rpm),
        "appimage" => Ok(PackageType::AppImage),
        "app" | "macos" => Ok(PackageType::MacOsBundle),
        "dmg" => Ok(PackageType::Dmg),
        "msi" => Ok(PackageType::WindowsMsi),
        "nsis" => Ok(PackageType::Nsis),
        _ => Err(ReleaseError::Cli(CliError::InvalidArguments {
            reason: format!("Unknown package type: '{}'. Valid: deb, rpm, appimage, app, dmg, msi, nsis", platform_str),
        })),
    }
}

/// Print bundle creation summary
pub(super) fn print_bundle_summary(artifacts: &[crate::bundler::BundledArtifact], config: &RuntimeConfig) {
    if artifacts.is_empty() {
        config.warning_println("No artifacts were created");
        return;
    }

    config.success_println(&format!("Created {} package(s)", artifacts.len()));

    for artifact in artifacts {
        config.println(&format!("\n  {:?}:", artifact.package_type));
        for path in &artifact.paths {
            let size_mb = artifact.size as f64 / 1_048_576.0;
            config.println(&format!("    📦 {} ({:.2} MB)", path.display(), size_mb));
        }
        config.println(&format!("    🔐 SHA256: {}", artifact.checksum));
    }
}

/// Upload bundles to GitHub release
pub(super) async fn upload_bundles_to_github(
    workspace: &crate::workspace::WorkspaceInfo,
    artifacts: &[crate::bundler::BundledArtifact],
    github_repo: Option<&str>,
    git_manager: &GitManager,
    config: &RuntimeConfig,
) -> Result<()> {
    use super::super::helpers::{parse_github_repo_string, detect_github_repo};

    config.println("📤 Uploading artifacts to GitHub...");

    // Parse owner/repo
    let (owner, repo) = if let Some(repo_str) = github_repo {
        parse_github_repo_string(repo_str)?
    } else {
        // Detect from git remote origin
        detect_github_repo(git_manager).await?
    };

    // Get version from workspace
    let version = workspace.packages.values().next()
        .ok_or_else(|| ReleaseError::Cli(CliError::InvalidArguments {
            reason: "No workspace members found".to_string(),
        }))?
        .version
        .clone();

    // Initialize GitHub manager
    let github_config = crate::github::GitHubReleaseConfig {
        owner: owner.clone(),
        repo: repo.clone(),
        draft: false,
        prerelease_for_zero_versions: true,
        notes: None,
        token: None, // Will read from GH_TOKEN or GITHUB_TOKEN env var
    };

    let github_manager = crate::github::GitHubReleaseManager::new(github_config)?;

    // Create release if doesn't exist, or get existing
    let tag_name = format!("v{}", version);
    config.verbose_println(&format!("Looking for release {}", tag_name));

    // Get or create release
    let client = github_manager.client().inner().clone();
    let release = kodegen_tools_github::get_release_by_tag(
        client.clone(),
        &owner,
        &repo,
        &tag_name,
    ).await
        .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
            command: "get_release_by_tag".to_string(),
            reason: e.to_string(),
        }))?;

    let release_id = if let Some(existing_release) = release {
        config.verbose_println(&format!("Found existing release: {}", existing_release.html_url));
        existing_release.id.0
    } else {
        // Create new release
        config.println(&format!("Creating release {}", tag_name));

        // Get current commit SHA
        let commit_sha = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|e| ReleaseError::Cli(CliError::ExecutionFailed {
                command: "git_rev_parse".to_string(),
                reason: e.to_string(),
            }))?;

        if !commit_sha.status.success() {
            return Err(ReleaseError::Cli(CliError::ExecutionFailed {
                command: "git_rev_parse".to_string(),
                reason: "Failed to get current commit SHA".to_string(),
            }));
        }

        let sha = String::from_utf8_lossy(&commit_sha.stdout).trim().to_string();

        // Parse version string to semver::Version
        let semver_version = semver::Version::parse(&version.to_string())
            .map_err(|e| ReleaseError::Cli(CliError::InvalidArguments {
                reason: format!("Invalid version '{}': {}", version, e),
            }))?;

        let result = github_manager.create_release(
            &semver_version,
            &sha,
            Some(format!("Release {}", version)),
        ).await?;

        config.success_println(&format!("Created release: {}", result.html_url));
        result.release_id
    };

    // Collect all artifact paths
    let mut all_paths = Vec::new();
    for artifact in artifacts {
        all_paths.extend(artifact.paths.clone());
    }

    // Upload artifacts
    config.verbose_println(&format!("Uploading {} files", all_paths.len()));
    let uploaded_urls = github_manager.upload_artifacts(release_id, &all_paths, config).await?;

    config.success_println(&format!("Uploaded {} artifact(s) to GitHub", uploaded_urls.len()));
    for url in &uploaded_urls {
        config.println(&format!("  📦 {}", url));
    }

    Ok(())
}
