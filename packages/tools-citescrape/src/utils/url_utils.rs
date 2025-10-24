//! URL and path manipulation utilities.
//!
//! This module provides functions for working with URLs and file paths
//! in the context of web crawling and mirroring.

use anyhow::Result;
use std::path::{Path, PathBuf};
use url::Url;

/// Extract a URI from a path, stripping the prefix and handling parent directory
pub async fn get_uri_from_path(path: &Path, output_dir: &Path) -> Result<String> {
    let result = path
        .strip_prefix(output_dir)
        .map_err(|e| anyhow::anyhow!("Failed to strip prefix: {e}"))?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path encoding"))?
        .replace('\\', "/");

    Ok(result)
}

/// Get the mirror path for a URL, preserving the domain and path structure
pub async fn get_mirror_path(url: &str, output_dir: &Path, filename: &str) -> Result<PathBuf> {
    let url = Url::parse(url).map_err(|e| anyhow::anyhow!("Failed to parse URL: {e}"))?;
    let domain = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;
    let path = if url.path() == "/" {
        PathBuf::new()
    } else {
        PathBuf::from(url.path().trim_start_matches('/'))
    };

    let mirror_path = output_dir.join(domain).join(path).join(filename);

    Ok(mirror_path)
}

/// Check if a URL is valid
#[must_use]
pub fn is_valid_url(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }

    // Skip data URLs, javascript URLs, and other non-http schemes
    if url.starts_with("data:") || url.starts_with("javascript:") || url.starts_with("mailto:") {
        return false;
    }

    match url::Url::parse(url) {
        Ok(parsed) => {
            matches!(parsed.scheme(), "http" | "https")
        }
        Err(_) => false,
    }
}

/// Ensure a .gitignore file exists in the domain directory
///
/// Creates a .gitignore file with `*` and `!.gitignore` patterns to exclude
/// all crawled content from version control while keeping the directory structure visible.
///
/// # Arguments
///
/// * `mirror_path` - Full path to a file in the domain (e.g., `output_dir/domain.com/path/file`)
/// * `output_dir` - Base output directory
///
/// # Returns
///
/// * `Result<()>` - Success or error
pub async fn ensure_domain_gitignore(mirror_path: &Path, output_dir: &Path) -> Result<()> {
    // Extract domain directory: strip output_dir, take first component
    let relative_path = mirror_path
        .strip_prefix(output_dir)
        .map_err(|e| anyhow::anyhow!("Failed to strip output_dir prefix: {e}"))?;

    let domain = relative_path
        .components()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Path has no domain component"))?;

    let domain_dir = output_dir.join(domain);
    let gitignore_path = domain_dir.join(".gitignore");

    // FIX #1: Ensure domain directory exists first (idempotent)
    tokio::fs::create_dir_all(&domain_dir)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create domain directory: {e}"))?;

    // FIX #2: Check if .gitignore exists, propagate errors instead of swallowing
    if tokio::fs::try_exists(&gitignore_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to check .gitignore existence: {e}"))?
    {
        return Ok(());
    }

    // Create .gitignore with ignore all except self pattern
    let gitignore_content = "*\n!.gitignore\n";

    tokio::fs::write(&gitignore_path, gitignore_content)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to write .gitignore: {e}"))?;

    log::debug!("Created .gitignore in {}", domain_dir.display());

    Ok(())
}
