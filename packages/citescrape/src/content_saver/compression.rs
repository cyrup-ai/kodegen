use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use flate2::{Compression, GzBuilder};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// Threshold for using spawn_blocking to prevent blocking the async runtime
/// Content larger than this will be compressed on a separate thread pool
const LARGE_CONTENT_THRESHOLD: usize = 1_048_576; // 1MB

/// Metadata stored in compressed files for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub etag: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub expires: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_modified: DateTime<Utc>,
    pub content_type: String,
}

/// Save content as a compressed file with cache metadata
pub async fn save_compressed_file(
    content: Vec<u8>,
    path: &Path,
    content_type: &str,
) -> Result<CacheMetadata> {
    let path = path.to_path_buf();
    let content_type = content_type.to_string();

    // Calculate XXHash for etag
    let hash = xxhash_rust::xxh3::xxh3_64(&content);
    let etag = format!("\"{:x}\"", hash);

    // Set cache control headers
    let now = Utc::now();
    let expires = now + Duration::seconds(7 * 24 * 60 * 60); // Cache for 7 days

    let metadata = CacheMetadata {
        etag,
        expires,
        last_modified: now,
        content_type: content_type.clone(),
    };

    // Store metadata in gzip header comment
    // Validate metadata size to prevent truncation
    let metadata_json = serde_json::to_string(&metadata)?;
    if metadata_json.len() > 60000 {
        return Err(anyhow::anyhow!(
            "Metadata too large for gzip comment: {} bytes exceeds 60000 byte limit",
            metadata_json.len()
        ));
    }

    let gz_path = path.with_extension(format!(
        "{}.gz",
        path.extension().unwrap_or_default().to_str().unwrap_or("")
    ));

    let filename_str = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Missing filename"))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid filename encoding"))?
        .to_string();

    // For large content (>1MB), use spawn_blocking to avoid blocking the async runtime
    if content.len() > LARGE_CONTENT_THRESHOLD {
        let gz_path_clone = gz_path.clone();
        let filename_clone = filename_str.clone();
        let metadata_json_clone = metadata_json.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let file = std::fs::File::create(&gz_path_clone)?;
            let mut gz = GzBuilder::new()
                .filename(filename_clone)
                .comment(metadata_json_clone)
                .write(file, Compression::new(3)); // Fast compression level
            gz.write_all(&content)?;
            gz.finish()?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("Spawn blocking join error: {}", e))??;
    } else {
        let file = std::fs::File::create(&gz_path)?;
        let mut gz = GzBuilder::new()
            .filename(filename_str)
            .comment(metadata_json)
            .write(file, Compression::new(3)); // Fast compression level
        gz.write_all(&content)?;
        gz.finish()?;
    }

    Ok(metadata)
}
