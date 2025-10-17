use anyhow::Result;
use std::sync::Arc;

use crate::runtime::{spawn_async, AsyncTask};
use crate::page_extractor::schema::ResourceInfo;
use crate::utils::get_mirror_path;

use super::compression::save_compressed_file;
use super::await_with_timeout;

/// Save HTML content after inlining all resources
pub fn save_html_content(
    html_content: String, 
    url: String, 
    output_dir: std::path::PathBuf,
    max_inline_image_size_bytes: Option<usize>,
    rate_rps: Option<f64>,
    on_result: impl FnOnce(Result<(), anyhow::Error>) + Send + 'static,
) -> AsyncTask<()> {
    
    spawn_async(async move {
        let result = async {
            // Wrap html_content in Arc to avoid multiple expensive clones
            let html_arc = Arc::new(html_content);
            
            // Start both operations in parallel
            let config = crate::inline_css::InlineConfig::default();
            let (inline_tx, inline_rx) = tokio::sync::oneshot::channel();
            let (path_tx, path_rx) = tokio::sync::oneshot::channel();
            
            let url_for_inline_log = url.clone();
            let html_for_inline = Arc::clone(&html_arc);
            let inline_task = crate::inline_css::inline_all_resources((*html_for_inline).clone(), url.clone(), &config, max_inline_image_size_bytes, rate_rps, move |result| {
                super::log_send_error::<crate::inline_css::InliningResult, anyhow::Error>(
                    inline_tx.send(result),
                    "inline_all_resources",
                    &url_for_inline_log
                );
            });
            
            let url_for_path = url.clone();
            let path_task = get_mirror_path(&url, &output_dir, "index.html", move |result| {
                super::log_send_error::<std::path::PathBuf, anyhow::Error>(
                    path_tx.send(result),
                    "get_mirror_path",
                    &url_for_path
                );
            });
            
            // Keep guards alive
            let _inline_guard = crate::runtime::TaskGuard::new(inline_task, "inline_all_resources");
            let _path_guard = crate::runtime::TaskGuard::new(path_task, "get_mirror_path_html");
            
            // Wait for both in parallel
            let (inline_result, path_result) = tokio::join!(
                await_with_timeout(inline_rx, 120, "inline resources"),
                await_with_timeout(path_rx, 30, "mirror path resolution")
            );
            
            let inlined_html = match inline_result {
                Ok(result) => match result {
                    Ok(inlined) => {
                        log::info!("Successfully inlined {} resources for: {} ({} failures)", 
                            inlined.successes, url, inlined.failures.len());
                        inlined.html
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to inline resources for {}: {}, using original HTML",
                            url,
                            e
                        );
                        (*html_arc).clone()
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to receive inline result for {}: {}, using original HTML",
                        url,
                        e
                    );
                    (*html_arc).clone()
                }
            };
            
            let path = path_result??;
            tokio::fs::create_dir_all(
                path.parent()
                    .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
            ).await?;
            
            // Use the converted save_compressed_file
            let (compress_tx, compress_rx) = tokio::sync::oneshot::channel();
            let url_for_compress = url.clone();
            let _compress_guard = save_compressed_file(
                inlined_html.into_bytes(), 
                &path, 
                "text/html",
                move |result| {
                    super::log_send_error::<super::CacheMetadata, anyhow::Error>(
                        compress_tx.send(result),
                        "save_compressed_file",
                        &url_for_compress
                    );
                }
            );
            let _metadata = await_with_timeout(compress_rx, 60, "compress HTML").await??;
            
            Ok(())
        }.await;
        
        on_result(result);
    })
}

/// Save HTML content with resource information for inlining
pub fn save_html_content_with_resources(
    html_content: &str,
    url: String,
    output_dir: std::path::PathBuf,
    resources: &ResourceInfo,
    max_inline_image_size_bytes: Option<usize>,
    rate_rps: Option<f64>,
    on_result: impl FnOnce(Result<(), anyhow::Error>) + Send + 'static,
) -> AsyncTask<()> {
    let html_content = html_content.to_string();
    let resources = resources.clone();
    
    spawn_async(async move {
        let result = async {
            // Wrap html_content in Arc to avoid multiple expensive clones
            let html_arc = Arc::new(html_content);
            
            // Start both operations in parallel
            let config = crate::inline_css::InlineConfig::default();
            let (html_res_inline_tx, html_res_inline_rx) = tokio::sync::oneshot::channel();
            let (html_res_path_tx, html_res_path_rx) = tokio::sync::oneshot::channel();
            
            let url_for_inline_res = url.clone();
            let html_for_inline_res = Arc::clone(&html_arc);
            let inline_res_task = crate::inline_css::inline_resources_from_info((*html_for_inline_res).clone(), url.clone(), &config, resources.clone(), max_inline_image_size_bytes, rate_rps, move |result| {
                super::log_send_error::<crate::inline_css::InliningResult, anyhow::Error>(
                    html_res_inline_tx.send(result),
                    "inline_resources_from_info",
                    &url_for_inline_res
                );
            });
            
            let url_for_path_res = url.clone();
            let path_res_task = get_mirror_path(&url, &output_dir, "index.html", move |result| {
                super::log_send_error::<std::path::PathBuf, anyhow::Error>(
                    html_res_path_tx.send(result),
                    "get_mirror_path",
                    &url_for_path_res
                );
            });
            
            // Keep guards alive
            let _inline_res_guard = crate::runtime::TaskGuard::new(inline_res_task, "inline_resources_from_info");
            let _path_res_guard = crate::runtime::TaskGuard::new(path_res_task, "get_mirror_path_html_res");
            
            // Wait for both in parallel
            let (inline_result, path_result) = tokio::join!(
                await_with_timeout(html_res_inline_rx, 120, "inline resources from info"),
                await_with_timeout(html_res_path_rx, 30, "mirror path resolution for HTML with resources")
            );
            
            let inlined_html = match inline_result {
                Ok(result) => match result {
                    Ok(inlined) => {
                        log::info!("Successfully inlined {} resources for: {} ({} failures)", 
                            inlined.successes, url, inlined.failures.len());
                        inlined.html
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to inline resources for {}: {}, using original HTML",
                            url,
                            e
                        );
                        (*html_arc).clone()
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to receive inline result for {}: {}, using original HTML",
                        url,
                        e
                    );
                    (*html_arc).clone()
                }
            };
            
            let path = path_result??;
            tokio::fs::create_dir_all(
                path.parent()
                    .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
            ).await?;
            
            let (html_res_compress_tx, html_res_compress_rx) = tokio::sync::oneshot::channel();
            let url_for_compress_res = url.clone();
            let _compress_res_guard = save_compressed_file(
                inlined_html.into_bytes(), 
                &path, 
                "text/html",
                move |result| {
                    super::log_send_error::<super::CacheMetadata, anyhow::Error>(
                        html_res_compress_tx.send(result),
                        "save_compressed_file",
                        &url_for_compress_res
                    );
                }
            );
            let _metadata = await_with_timeout(html_res_compress_rx, 60, "compress HTML with resources").await??;
            
            Ok(())
        }.await;
        
        on_result(result);
    })
}
