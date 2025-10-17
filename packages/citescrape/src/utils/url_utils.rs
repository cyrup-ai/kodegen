//! URL and path manipulation utilities.
//!
//! This module provides functions for working with URLs and file paths
//! in the context of web crawling and mirroring.

use anyhow::Result;
use std::path::{Path, PathBuf};
use url::Url;
use crate::runtime::{spawn_async, AsyncTask};

/// Extract a URI from a path, stripping the prefix and handling parent directory
pub fn get_uri_from_path(
    path: &Path,
    output_dir: &Path,
    on_result: impl FnOnce(Result<String>) + Send + 'static,
) -> AsyncTask<()> {
    let path = path.to_path_buf();
    let output_dir = output_dir.to_path_buf();
    
    spawn_async(async move {
        let result: Result<String> = (|| -> Result<String> {
            let result = path
                .strip_prefix(&output_dir)
                .map_err(|e| anyhow::anyhow!("Failed to strip prefix: {}", e))?
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid path encoding"))?
                .replace('\\', "/")
                .to_string();

            Ok(result)
        })();
        
        on_result(result);
    })
}

/// Get the mirror path for a URL, preserving the domain and path structure
pub fn get_mirror_path(
    url: &str,
    output_dir: &Path,
    filename: &str,
    on_result: impl FnOnce(Result<PathBuf>) + Send + 'static,
) -> AsyncTask<()> {
    let url = url.to_string();
    let output_dir = output_dir.to_path_buf();
    let filename = filename.to_string();
    
    spawn_async(async move {
        let result: Result<PathBuf> = (|| -> Result<PathBuf> {
            let url = Url::parse(&url).map_err(|e| anyhow::anyhow!("Failed to parse URL: {}", e))?;
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
        })();
        
        on_result(result);
    })
}

/// Check if a URL is valid
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