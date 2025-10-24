//! Shared helper functions for command execution.

use crate::error::{CliError, ReleaseError, Result};
use crate::git::GitManager;
use crate::workspace::WorkspaceInfo;

/// Parse GitHub repository string into owner/repo tuple
#[allow(dead_code)]
pub(super) fn parse_github_repo(repo_str: Option<&str>) -> Result<(String, String)> {
    let repo = repo_str.ok_or_else(|| {
        ReleaseError::Cli(CliError::InvalidArguments {
            reason: "--github-repo is required when --github-release is used. Format: owner/repo"
                .to_string(),
        })
    })?;

    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        return Err(ReleaseError::Cli(CliError::InvalidArguments {
            reason: format!(
                "Invalid GitHub repository format: '{}'. Expected: owner/repo",
                repo
            ),
        }));
    }

    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Parse GitHub repo string "owner/repo"
pub(super) fn parse_github_repo_string(repo_str: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = repo_str.split('/').collect();
    if parts.len() != 2 {
        return Err(ReleaseError::Cli(CliError::InvalidArguments {
            reason: format!(
                "Invalid GitHub repository format: '{}'. Expected: owner/repo",
                repo_str
            ),
        }));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Parse GitHub owner/repo from git remote URL
/// Supports: git@github.com:owner/repo.git and https://github.com/owner/repo.git
pub(super) fn parse_github_url(url: &str) -> Option<(String, String)> {
    // Handle git@github.com:owner/repo.git (with or without leading slash)
    if let Some(ssh_part) = url.strip_prefix("git@github.com:") {
        // Remove leading slash if present (malformed URL like git@github.com:/owner/repo)
        let ssh_part = ssh_part.strip_prefix('/').unwrap_or(ssh_part);
        let repo_part = ssh_part.strip_suffix(".git").unwrap_or(ssh_part);
        let parts: Vec<&str> = repo_part.split('/').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Handle https://github.com/owner/repo.git
    if url.contains("github.com/")
        && let Some(path) = url.split("github.com/").nth(1)
    {
        let repo_part = path.strip_suffix(".git").unwrap_or(path);
        let parts: Vec<&str> = repo_part.split('/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    None
}

/// Detect GitHub repo from git remote origin using GitManager
pub(super) async fn detect_github_repo(git_manager: &GitManager) -> Result<(String, String)> {
    let remotes = git_manager.remotes().await?;

    // Find origin remote
    let origin = remotes.iter().find(|r| r.name == "origin").ok_or_else(|| {
        ReleaseError::Cli(CliError::InvalidArguments {
            reason:
                "No 'origin' remote configured. Git requires origin for push/pull/tag operations."
                    .to_string(),
        })
    })?;

    // Parse GitHub URL from origin
    parse_github_url(&origin.fetch_url).ok_or_else(|| {
        ReleaseError::Cli(CliError::InvalidArguments {
            reason: format!(
                "Origin remote is not a GitHub repository: {}",
                origin.fetch_url
            ),
        })
    })
}

/// Create distributable bundles for the release
pub(super) fn create_bundles(
    workspace: &WorkspaceInfo,
    version: &semver::Version,
    _config: &crate::cli::RuntimeConfig,
) -> Result<Vec<crate::bundler::BundledArtifact>> {
    use crate::bundler::{BundleSettings, Bundler, PackageSettings, SettingsBuilder};
    use std::path::PathBuf;

    // Extract product name from first package
    let product_name = workspace
        .packages
        .values()
        .next()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "app".to_string());

    // Extract description from workspace config
    let description = workspace
        .workspace_config
        .package
        .as_ref()
        .and_then(|p| p.other.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or("Rust application")
        .to_string();

    // Build package settings with workspace metadata
    let package_settings = PackageSettings {
        product_name,
        version: version.to_string(),
        description,
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

    // Configure bundle settings with icons and post-install scripts
    // Note: macOS .app bundles don't support post-install scripts - kodegen_install
    // must run automatically on first app launch instead
    use crate::bundler::{DebianSettings, RpmSettings};
    let bundle_settings = BundleSettings {
        identifier: Some(format!("ai.kodegen.{}", package_settings.product_name)),
        icon: Some(icon_paths),
        deb: DebianSettings {
            post_install_script: Some(PathBuf::from("packages/bundler-release/postinst.deb.sh")),
            ..Default::default()
        },
        rpm: RpmSettings {
            post_install_script: Some(PathBuf::from("packages/bundler-release/postinst.rpm.sh")),
            ..Default::default()
        },
        ..Default::default()
    };

    // Configure all required binaries for the bundle
    // kodegen_install runs first to setup system and register kodegend daemon
    use crate::bundler::BundleBinary;
    let binaries = vec![
        BundleBinary::new("kodegen_install".to_string(), true), // primary installer (runs first)
        BundleBinary::new("kodegend".to_string(), false),       // service daemon
        BundleBinary::new("kodegen".to_string(), false),        // main MCP server
    ];

    // Use SettingsBuilder to create Settings
    let settings = SettingsBuilder::new()
        .project_out_directory(workspace.root.join("target/release"))
        .package_settings(package_settings)
        .bundle_settings(bundle_settings)
        .binaries(binaries)
        .build()
        .map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: "build_settings".to_string(),
                reason: e.to_string(),
            })
        })?;

    // Now Bundler::new() gets the correct Settings type
    let bundler = Bundler::new(settings).map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "create_bundler".to_string(),
            reason: e.to_string(),
        })
    })?;

    bundler.bundle().map_err(|e| {
        ReleaseError::Cli(CliError::ExecutionFailed {
            command: "bundle_artifacts".to_string(),
            reason: e.to_string(),
        })
    })
}
