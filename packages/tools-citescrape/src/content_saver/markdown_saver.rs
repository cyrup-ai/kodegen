use anyhow::Result;
use std::sync::Arc;

use crate::search::IndexingSender;
use crate::search::MessagePriority;
use crate::utils::{ensure_domain_gitignore, get_mirror_path};

use super::compression::save_compressed_file;

/// Save markdown content to disk with optional search indexing
///
/// # Arguments
///
/// * `markdown_content` - The markdown text to save
/// * `url` - Source URL (used for path generation and indexing metadata)
/// * `output_dir` - Base directory for mirrored content
/// * `priority` - Indexing priority for search
/// * `indexing_sender` - Optional channel for triggering search indexing
///
/// # Returns
///
/// * `Result<()>` - Result of the save operation
pub async fn save_markdown_content(
    markdown_content: String,
    url: String,
    output_dir: std::path::PathBuf,
    priority: MessagePriority,
    indexing_sender: Option<Arc<IndexingSender>>,
    compress: bool,
) -> Result<()> {
    let path = get_mirror_path(&url, &output_dir, "index.md").await?;

    // Ensure .gitignore exists in domain directory
    ensure_domain_gitignore(&path, &output_dir).await?;

    // Ensure parent directory exists
    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    )
    .await?;

    // save_compressed_file returns the actual saved path (.gz if compressed, plain otherwise)
    let (saved_path, metadata) = save_compressed_file(
        markdown_content.into_bytes(),
        &path,
        "text/markdown",
        compress,
    )
    .await?;

    // Trigger search indexing if sender provided
    if let Some(sender) = indexing_sender {
        use imstr::ImString;

        let url_imstr = ImString::from(url.clone());
        let path_for_indexing = saved_path.clone();
        let url_for_callback = url.clone();

        let index_result =
            sender.add_or_update(url_imstr, path_for_indexing, priority, move |result| {
                if let Err(e) = result {
                    log::warn!("Indexing failed for {url_for_callback}: {e}");
                }
            });

        if let Err(e) = index_result.await {
            log::warn!("Failed to queue indexing for {url}: {e}");
            // Don't fail the save operation if indexing fails
        }
    }

    log::info!(
        "Saved markdown for {} to {} (etag: {})",
        url,
        saved_path.display(),
        metadata.etag
    );

    Ok(())
}
