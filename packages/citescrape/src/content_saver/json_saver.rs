use anyhow::Result;

use crate::runtime::{spawn_async, AsyncTask};
use crate::utils::get_mirror_path;

use super::compression::save_compressed_file;
use super::await_with_timeout;

/// Save JSON data
pub fn save_json_data(
    data: serde_json::Value, 
    url: String, 
    output_dir: std::path::PathBuf,
    on_result: impl FnOnce(Result<(), anyhow::Error>) + Send + 'static,
) -> AsyncTask<()> {

    spawn_async(async move {
        let result = async {
            // Start both operations in parallel
            let (json_path_tx, json_path_rx) = tokio::sync::oneshot::channel();
            let url_for_path = url.clone();
            let path_task = get_mirror_path(&url, &output_dir, "index.json", move |result| {
                super::log_send_error::<std::path::PathBuf, anyhow::Error>(
                    json_path_tx.send(result),
                    "get_mirror_path",
                    &url_for_path
                );
            });
            let _path_guard = crate::runtime::TaskGuard::new(path_task, "get_mirror_path_json");
            
            // Serialize JSON on blocking thread pool
            let json_handle = tokio::task::spawn_blocking(move || {
                serde_json::to_string_pretty(&data)
            });
            
            // Wait for both
            let (path_result, json_result) = tokio::join!(
                await_with_timeout(json_path_rx, 30, "mirror path resolution for JSON"),
                json_handle
            );
            
            let path = path_result??;
            let json_str = json_result
                .map_err(|e| anyhow::anyhow!("JSON serialization task panicked: {}", e))??;
            
            tokio::fs::create_dir_all(
                path.parent()
                    .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
            ).await?;
            
            let (json_compress_tx, json_compress_rx) = tokio::sync::oneshot::channel();
            let url_for_compress = url.clone();
            let _compress_guard = save_compressed_file(
                json_str.into_bytes(), 
                &path, 
                "application/json",
                move |result| {
                    super::log_send_error::<super::CacheMetadata, anyhow::Error>(
                        json_compress_tx.send(result),
                        "save_compressed_file",
                        &url_for_compress
                    );
                }
            );
            let _metadata = await_with_timeout(json_compress_rx, 60, "compress JSON").await??;
            
            Ok(())
        }.await;

        on_result(result);
    })
}

/// Save page data as JSON
pub fn save_page_data(
    page_data: std::sync::Arc<crate::page_extractor::schema::PageData>,
    url: String,
    output_dir: std::path::PathBuf,
    on_result: impl FnOnce(Result<(), anyhow::Error>) + Send + 'static,
) -> AsyncTask<()> {
    
    spawn_async(async move {
        let result = async {
            // Start both operations in parallel
            let (page_path_tx, page_path_rx) = tokio::sync::oneshot::channel();
            let url_for_page_path = url.clone();
            let page_path_task = get_mirror_path(&url, &output_dir, "index.json", move |result| {
                super::log_send_error::<std::path::PathBuf, anyhow::Error>(
                    page_path_tx.send(result),
                    "get_mirror_path",
                    &url_for_page_path
                );
            });
            let _page_path_guard = crate::runtime::TaskGuard::new(page_path_task, "get_mirror_path_page_data");
            
            // Serialize PageData on blocking thread pool (in parallel with path resolution)
            let json_handle = tokio::task::spawn_blocking(move || {
                serde_json::to_string_pretty(&*page_data)
            });
            
            // Wait for both
            let (path_result, json_result) = tokio::join!(
                await_with_timeout(page_path_rx, 30, "mirror path resolution for page data JSON"),
                json_handle
            );
            
            let path = path_result??;
            let json_content = json_result
                .map_err(|e| anyhow::anyhow!("PageData serialization task panicked: {}", e))??;
            
            let (page_compress_tx, page_compress_rx) = tokio::sync::oneshot::channel();
            let url_for_page_compress = url.clone();
            let _page_compress_guard = save_compressed_file(
                json_content.into_bytes(), 
                &path, 
                "application/json",
                move |result| {
                    super::log_send_error::<super::CacheMetadata, anyhow::Error>(
                        page_compress_tx.send(result),
                        "save_compressed_file",
                        &url_for_page_compress
                    );
                }
            );
            let _metadata = await_with_timeout(page_compress_rx, 60, "compress page data").await??;
            
            Ok(())
        }.await;
        
        on_result(result);
    })
}
