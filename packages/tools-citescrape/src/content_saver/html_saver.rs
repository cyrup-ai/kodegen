use anyhow::Result;

use crate::page_extractor::schema::ResourceInfo;
use crate::utils::{ensure_domain_gitignore, get_mirror_path};

use super::compression::save_compressed_file;

/// Save HTML content after inlining all resources
pub async fn save_html_content(
    html_content: String,
    url: String,
    output_dir: std::path::PathBuf,
    max_inline_image_size_bytes: Option<usize>,
    rate_rps: Option<f64>,
) -> Result<()> {
    let config = crate::inline_css::InlineConfig::default();

    // get_mirror_path is async, await it
    let path = get_mirror_path(&url, &output_dir, "index.html").await?;

    // Ensure .gitignore exists in domain directory
    ensure_domain_gitignore(&path, &output_dir).await?;

    // inline_all_resources is async
    let inline_result = crate::inline_css::inline_all_resources(
        html_content.clone(),
        url.clone(),
        &config,
        max_inline_image_size_bytes,
        rate_rps,
    )
    .await;

    let inlined_html = match inline_result {
        Ok(inlined) => {
            log::info!(
                "Successfully inlined {} resources for: {} ({} failures)",
                inlined.successes,
                url,
                inlined.failures.len()
            );
            inlined.html
        }
        Err(e) => {
            log::warn!("Failed to inline resources for {url}: {e}, using original HTML");
            html_content
        }
    };

    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    )
    .await?;

    // save_compressed_file is now async
    let (_saved_path, _metadata) =
        save_compressed_file(inlined_html.into_bytes(), &path, "text/html", false).await?;

    Ok(())
}

/// Save HTML content with resource information for inlining
pub async fn save_html_content_with_resources(
    html_content: &str,
    url: String,
    output_dir: std::path::PathBuf,
    resources: &ResourceInfo,
    max_inline_image_size_bytes: Option<usize>,
    rate_rps: Option<f64>,
) -> Result<()> {
    let html_content = html_content.to_string();
    let resources = resources.clone();

    // Get mirror path first (async)
    let path = get_mirror_path(&url, &output_dir, "index.html").await?;

    // Ensure .gitignore exists in domain directory
    ensure_domain_gitignore(&path, &output_dir).await?;

    // Then inline resources (async)
    let config = crate::inline_css::InlineConfig::default();
    let inline_future = crate::inline_css::inline_resources_from_info(
        html_content.clone(),
        url.clone(),
        &config,
        resources,
        max_inline_image_size_bytes,
        rate_rps,
    )
    .await;

    let inlined_html = match inline_future {
        Ok(inlined) => {
            log::info!(
                "Successfully inlined {} resources for: {} ({} failures)",
                inlined.successes,
                url,
                inlined.failures.len()
            );
            inlined.html
        }
        Err(e) => {
            log::warn!("Failed to inline resources for {url}: {e}, using original HTML");
            html_content
        }
    };

    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    )
    .await?;

    // save_compressed_file is now async
    let (_saved_path, _metadata) =
        save_compressed_file(inlined_html.into_bytes(), &path, "text/html", false).await?;

    Ok(())
}
