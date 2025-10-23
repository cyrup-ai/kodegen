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
