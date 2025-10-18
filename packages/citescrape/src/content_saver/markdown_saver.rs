use anyhow::Result;
use std::sync::Arc;

use crate::search::IndexingSender;
use crate::search::MessagePriority;
use crate::utils::get_mirror_path;

use super::compression::save_compressed_file;
use super::await_with_timeout;

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
) -> Result<()> {
    // Determine save path
    let (path_tx, path_rx) = tokio::sync::oneshot::channel();
    let url_for_path = url.clone();
    let path_task = get_mirror_path(&url, &output_dir, "index.md", move |result| {
        super::log_send_error::<std::path::PathBuf, anyhow::Error>(
            path_tx.send(result),
            "get_mirror_path",
            &url_for_path
        );
    });
    
    let _path_guard = crate::runtime::TaskGuard::new(path_task, "get_mirror_path_markdown");
    let path = await_with_timeout(path_rx, 30, "mirror path resolution for markdown").await??;
    
    // Ensure parent directory exists
    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    ).await?;
    
    // Save compressed markdown
    let (compress_tx, compress_rx) = tokio::sync::oneshot::channel();
    let url_for_compress = url.clone();
    let _compress_guard = save_compressed_file(
        markdown_content.into_bytes(),
        &path,
        "text/markdown",
        move |result| {
            super::log_send_error::<super::CacheMetadata, anyhow::Error>(
                compress_tx.send(result),
                "save_compressed_file",
                &url_for_compress
            );
        }
    );
    
    let metadata = await_with_timeout(compress_rx, 60, "compress markdown").await??;
    
    // Trigger search indexing if sender provided
    if let Some(sender) = indexing_sender {
        use imstr::ImString;
        
        let url_imstr = ImString::from(url.clone());
        let path_for_indexing = path.clone();
        let url_for_callback = url.clone();
        
        let index_result = sender.add_or_update(
            url_imstr,
            path_for_indexing,
            priority,
            move |result| {
                if let Err(e) = result {
                    log::warn!("Indexing failed for {}: {}", url_for_callback, e);
                }
            }
        );
        
        if let Err(e) = index_result.await {
            log::warn!("Failed to queue indexing for {}: {}", url, e);
            // Don't fail the save operation if indexing fails
        }
    }
    
    log::info!("Saved markdown for {} to {} (etag: {})", 
        url, path.display(), metadata.etag);
    
    Ok(())
}
