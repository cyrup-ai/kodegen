//! GitHub Release management for coordinating release operations

use crate::error::{CliError, ReleaseError, Result};
use bytes::Bytes;
use kodegen_tools_github::{GitHubClient, GitHubReleaseOptions};
use semver::Version;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Configuration for GitHub releases
#[derive(Debug, Clone)]
pub struct GitHubReleaseConfig {
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Whether to create draft releases
    pub draft: bool,
    /// Whether to mark as pre-release for pre-1.0 versions
    pub prerelease_for_zero_versions: bool,
    /// Custom release notes
    pub notes: Option<String>,
    /// GitHub token (from environment or config)
    pub token: Option<String>,
}

impl Default for GitHubReleaseConfig {
    fn default() -> Self {
        Self {
            owner: String::new(),
            repo: String::new(),
            draft: false,
            prerelease_for_zero_versions: true,
            notes: None,
            token: None,
        }
    }
}

/// Result of GitHub release operation
#[derive(Debug, Clone)]
pub struct GitHubReleaseResult {
    /// Release ID
    pub release_id: u64,
    /// Release URL
    pub html_url: String,
    /// Whether this was a draft
    pub draft: bool,
    /// Whether this was a prerelease
    pub prerelease: bool,
    /// Duration of operation
    pub duration: Duration,
}

/// GitHub release manager
pub struct GitHubReleaseManager {
    /// GitHub client
    client: GitHubClient,
    /// Configuration
    config: GitHubReleaseConfig,
}

impl GitHubReleaseManager {
    /// Create new GitHub release manager
    pub fn new(config: GitHubReleaseConfig) -> Result<Self> {
        // Get token from config or environment
        let token = config.token.clone()
            .or_else(|| std::env::var("GH_TOKEN").ok())
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .ok_or_else(|| ReleaseError::Cli(CliError::InvalidArguments {
                reason: "GitHub token not provided. Set GH_TOKEN or GITHUB_TOKEN environment variable or use --github-token".to_string(),
            }))?;

        let client = GitHubClient::with_token(token).map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: "github_client_init".to_string(),
                reason: e.to_string(),
            })
        })?;

        Ok(Self { client, config })
    }

    /// Get reference to the GitHub client
    pub fn client(&self) -> &GitHubClient {
        &self.client
    }

    /// Create a GitHub release
    pub async fn create_release(
        &self,
        version: &Version,
        commit_sha: &str,
        release_notes: Option<String>,
    ) -> Result<GitHubReleaseResult> {
        let start_time = Instant::now();

        let tag_name = format!("v{}", version);

        // Determine if this should be a prerelease
        let is_prerelease = if self.config.prerelease_for_zero_versions {
            version.major == 0 || !version.pre.is_empty()
        } else {
            !version.pre.is_empty()
        };

        // Use provided release notes or custom notes from config
        let body = release_notes
            .or_else(|| self.config.notes.clone())
            .or_else(|| Some(format!("Release version {}", version)));

        let options = GitHubReleaseOptions {
            tag_name: tag_name.clone(),
            target_commitish: Some(commit_sha.to_string()),
            name: Some(format!("Release {}", version)),
            body,
            draft: self.config.draft,
            prerelease: is_prerelease,
        };

        let result = kodegen_tools_github::create_release(
            self.client.inner().clone(),
            &self.config.owner,
            &self.config.repo,
            options,
        )
        .await
        .map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: "create_github_release".to_string(),
                reason: e.to_string(),
            })
        })?;

        Ok(GitHubReleaseResult {
            release_id: result.id,
            html_url: result.html_url,
            draft: result.draft,
            prerelease: result.prerelease,
            duration: start_time.elapsed(),
        })
    }

    /// Delete a release (for rollback)
    pub async fn delete_release(&self, release_id: u64) -> Result<()> {
        kodegen_tools_github::delete_release(
            self.client.inner().clone(),
            &self.config.owner,
            &self.config.repo,
            release_id,
        )
        .await
        .map_err(|e| {
            ReleaseError::Cli(CliError::ExecutionFailed {
                command: "delete_github_release".to_string(),
                reason: e.to_string(),
            })
        })
    }

    /// Upload signed artifacts to release
    ///
    /// Reads artifact files and uploads them as release assets.
    /// Returns list of download URLs for the uploaded assets.
    pub async fn upload_artifacts(
        &self,
        release_id: u64,
        artifact_paths: &[PathBuf],
        runtime_config: &crate::cli::RuntimeConfig,
    ) -> Result<Vec<String>> {
        let mut uploaded_urls = Vec::new();

        for artifact_path in artifact_paths {
            // Extract filename for the asset
            let filename = artifact_path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    ReleaseError::Cli(CliError::InvalidArguments {
                        reason: format!("Invalid artifact filename: {:?}", artifact_path),
                    })
                })?;

            // Read file content
            let content = std::fs::read(artifact_path).map_err(|e| {
                ReleaseError::Cli(CliError::ExecutionFailed {
                    command: "read_artifact".to_string(),
                    reason: e.to_string(),
                })
            })?;

            // Create upload options
            let upload_options = kodegen_tools_github::UploadAssetOptions {
                release_id,
                asset_name: filename.to_string(),
                label: Some(create_artifact_label(filename)),
                content: Bytes::from(content),
                replace_existing: false, // Safer default - fails if asset exists
            };

            // Upload via GitHub client
            let asset = self
                .client
                .upload_release_asset(&self.config.owner, &self.config.repo, upload_options)
                .await
                .map_err(|e| ReleaseError::GitHub(e.to_string()))?;

            // Extract download URL from asset
            uploaded_urls.push(asset.browser_download_url.to_string());

            runtime_config.indent(&format!("✓ Uploaded: {} ({} bytes)", filename, asset.size));
        }

        Ok(uploaded_urls)
    }
}

/// Detect MIME type for bundle artifacts
///
/// Note: octocrab automatically detects content types from file extensions,
/// but we provide this for future extensibility and explicit documentation.
#[allow(dead_code)]
fn detect_bundle_content_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("deb") => "application/vnd.debian.binary-package",
        Some("rpm") => "application/x-rpm",
        Some("msi") => "application/x-msi",
        Some("exe") => "application/x-msdownload",
        Some("dmg") => "application/x-apple-diskimage",
        Some("app") => "application/x-apple-bundle",
        Some("AppImage") => "application/x-executable",
        Some("zip") => "application/zip",
        Some("tar") | Some("gz") | Some("tgz") => "application/gzip",
        _ => "application/octet-stream",
    }
}

/// Create descriptive label for artifact based on filename
fn create_artifact_label(filename: &str) -> String {
    // Extract architecture
    let arch = if filename.contains("aarch64") || filename.contains("arm64") {
        "ARM64"
    } else if filename.contains("x86_64") || filename.contains("amd64") {
        "x86_64"
    } else {
        "multi-arch"
    };

    // Extract platform
    let platform = if filename.contains("deb") {
        "Debian/Ubuntu"
    } else if filename.contains("rpm") {
        "RedHat/Fedora"
    } else if filename.contains("dmg") || filename.contains(".app") {
        "macOS"
    } else if filename.contains("msi") || filename.contains(".exe") {
        "Windows"
    } else if filename.contains("AppImage") {
        "Linux AppImage"
    } else {
        "Binary"
    };

    format!("kodegen {} - {}", platform, arch)
}
