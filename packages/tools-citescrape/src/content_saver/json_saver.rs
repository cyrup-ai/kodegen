use anyhow::Result;

use crate::utils::{ensure_domain_gitignore, get_mirror_path};

use super::compression::save_compressed_file;

/// Save JSON data
pub async fn save_json_data(
    data: serde_json::Value,
    url: String,
    output_dir: std::path::PathBuf,
) -> Result<()> {
    let path = get_mirror_path(&url, &output_dir, "index.json").await?;

    // Ensure .gitignore exists in domain directory
    ensure_domain_gitignore(&path, &output_dir).await?;

    // JSON serialization (keep spawn_blocking - CPU intensive)
    let json_str = tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&data))
        .await
        .map_err(|e| anyhow::anyhow!("JSON serialization task panicked: {e}"))??;

    // Create directory
    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    )
    .await?;

    // save_compressed_file is now async
    let (_saved_path, _metadata) =
        save_compressed_file(json_str.into_bytes(), &path, "application/json", false).await?;

    Ok(())
}

/// Save page data as JSON
pub async fn save_page_data(
    page_data: std::sync::Arc<crate::page_extractor::schema::PageData>,
    url: String,
    output_dir: std::path::PathBuf,
) -> Result<()> {
    let path = get_mirror_path(&url, &output_dir, "index.json").await?;

    // Ensure .gitignore exists in domain directory
    ensure_domain_gitignore(&path, &output_dir).await?;

    // PageData serialization (keep spawn_blocking - CPU intensive)
    let json_content =
        tokio::task::spawn_blocking(move || serde_json::to_string_pretty(&*page_data))
            .await
            .map_err(|e| anyhow::anyhow!("PageData serialization task panicked: {e}"))??;

    // Create parent directory
    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    )
    .await?;

    // save_compressed_file is now async
    let (_saved_path, _metadata) =
        save_compressed_file(json_content.into_bytes(), &path, "application/json", false).await?;

    Ok(())
}
