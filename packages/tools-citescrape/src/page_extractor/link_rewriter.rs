use anyhow::Result;
use lol_html::{HtmlRewriter, Settings, element};
use lru::LruCache;
use regex::Regex;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

/// Default capacity for URL mappings cache
///
/// 10,000 entries × 250 bytes ≈ 2.5 MB
/// Balances memory usage with link rewriting coverage
const DEFAULT_URL_CACHE_CAPACITY: usize = 10_000;

/// Internal state for `LinkRewriter` combining URL map and registration count
struct LinkRewriterState {
    /// LRU cache mapping absolute URLs to their local file paths
    ///
    /// Capacity-limited to prevent unbounded memory growth on large crawls.
    /// When at capacity, least recently accessed mappings are evicted.
    url_to_local: LruCache<String, String>,
    /// Count of registered URL→path mappings
    registration_count: usize,
}

/// Manages link rewriting for crawled pages
pub struct LinkRewriter {
    /// Shared state containing URL map and registration count
    state: Arc<Mutex<LinkRewriterState>>,
    /// Base output directory
    output_dir: String,
}

impl Clone for LinkRewriter {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            output_dir: self.output_dir.clone(),
        }
    }
}

impl LinkRewriter {
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        Self {
            state: Arc::new(Mutex::new(LinkRewriterState {
                url_to_local: LruCache::new(
                    NonZeroUsize::new(DEFAULT_URL_CACHE_CAPACITY)
                        .expect("DEFAULT_URL_CACHE_CAPACITY must be non-zero"),
                ),
                registration_count: 0,
            })),
            output_dir: output_dir.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Register a crawled URL with its local path
    pub async fn register_url(&self, url: &str, local_path: &str) {
        let mut state = self.state.lock().await;
        // LruCache::put returns evicted value if cache was at capacity
        let _evicted = state
            .url_to_local
            .put(url.to_string(), local_path.to_string());
        state.registration_count += 1;

        // Optional: Log when eviction occurs (helps understand cache pressure)
        if _evicted.is_some() {
            log::trace!("LinkRewriter cache at capacity, evicted old URL mapping");
        }
    }

    /// Get count of registered URLs (indicates link rewriting capability)
    pub async fn get_registration_count(&self) -> usize {
        let state = self.state.lock().await;
        state.registration_count
    }

    /// Check if URL is registered (available for local link rewriting)
    pub async fn is_registered(&self, url: &str) -> bool {
        let state = self.state.lock().await;
        state.url_to_local.contains(url)
    }

    /// Get local path for URL if registered
    pub async fn get_local_path(&self, url: &str) -> Option<String> {
        let mut state = self.state.lock().await;
        state.url_to_local.get(url).cloned()
    }

    /// Add data attributes to all links in HTML for tracking
    pub async fn mark_links_for_discovery(&self, html: &str, current_url: &str) -> Result<String> {
        let html = html.to_string();
        let current_url = current_url.to_string();

        let current_url_parsed = Url::parse(&current_url)?;
        let crawler_id = Uuid::new_v4().to_string();

        let mut output = Vec::new();

        // Clone values for closure capture
        let current_url_for_closure = current_url.clone();
        let crawler_id_for_closure = crawler_id.clone();

        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!("a[href]", move |el| {
                    let href = match el.get_attribute("href") {
                        Some(h) => h,
                        None => return Ok(()), // Skip if no href
                    };

                    // Skip non-http(s) links
                    if href.starts_with("mailto:")
                        || href.starts_with("javascript:")
                        || href.starts_with("tel:")
                        || href.starts_with("data:")
                    {
                        return Ok(());
                    }

                    // Convert to absolute URL
                    let absolute_url = match current_url_parsed.join(&href) {
                        Ok(url) => url,
                        Err(_) => return Ok(()), // Skip invalid URLs
                    };

                    // Add tracking attributes
                    el.set_attribute("data-crawler-id", &crawler_id_for_closure)?;
                    el.set_attribute("data-original-href", &href)?;
                    el.set_attribute("data-absolute-href", absolute_url.as_str())?;
                    el.set_attribute("data-crawler-current-url", &current_url_for_closure)?;

                    Ok(())
                })],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );

        rewriter
            .write(html.as_bytes())
            .map_err(|e| anyhow::anyhow!("HtmlRewriter error: {e}"))?;
        rewriter
            .end()
            .map_err(|e| anyhow::anyhow!("HtmlRewriter end error: {e}"))?;

        String::from_utf8(output)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in rewritten HTML: {e}"))
    }

    /// Rewrite all links in HTML to point to local files
    pub async fn rewrite_links(&self, html: String, current_url: String) -> Result<String> {
        let state = self.state.clone();
        let output_dir = self.output_dir.clone();

        let current_url_parsed = Url::parse(&current_url)?;

        let mut output = Vec::new();

        // Clone for closure capture
        let state_for_closure = state.clone();
        let current_url_for_closure = current_url.clone();
        let output_dir_for_closure = output_dir.clone();

        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!("a[href]", move |el| {
                    let href = match el.get_attribute("href") {
                        Some(h) => h,
                        None => return Ok(()),
                    };

                    // Skip special protocols
                    if href.starts_with("mailto:")
                        || href.starts_with("javascript:")
                        || href.starts_with('#')
                    {
                        return Ok(());
                    }

                    // Convert to absolute URL
                    let absolute_url = match current_url_parsed.join(&href) {
                        Ok(url) => url,
                        Err(_) => return Ok(()),
                    };

                    // Check if we have a local version
                    let local_path = {
                        match state_for_closure.try_lock() {
                            Ok(mut state_guard) => {
                                // LruCache::get needs mutable access to update LRU order
                                state_guard.url_to_local.get(absolute_url.as_str()).cloned()
                            }
                            Err(_) => None, // Lock contended, skip this link
                        }
                    };

                    if let Some(local_path) = local_path {
                        // Calculate relative path
                        match Self::calculate_relative_path_sync(
                            &current_url_for_closure,
                            &local_path,
                            &output_dir_for_closure,
                        ) {
                            Ok(relative_path) => {
                                el.set_attribute("href", &relative_path)?;
                            }
                            Err(e) => {
                                log::warn!("Failed to calculate relative path for {href}: {e}");
                            }
                        }
                    }
                    // If not crawled, leave as external link

                    Ok(())
                })],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );

        rewriter
            .write(html.as_bytes())
            .map_err(|e| anyhow::anyhow!("HtmlRewriter error: {e}"))?;
        rewriter
            .end()
            .map_err(|e| anyhow::anyhow!("HtmlRewriter end error: {e}"))?;

        String::from_utf8(output)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in rewritten HTML: {e}"))
    }

    /// Calculate relative path from one URL to another local path
    pub async fn calculate_relative_path(
        &self,
        from_url: String,
        to_local_path: String,
    ) -> Result<String> {
        let output_dir = self.output_dir.clone();
        Self::calculate_relative_path_sync(&from_url, &to_local_path, &output_dir)
    }

    /// Calculate relative path from source URL to target local path (synchronous)
    ///
    /// This is the core path calculation logic extracted for use in async contexts.
    ///
    /// # Arguments
    /// * `from_url` - The source page URL (e.g., "<https://example.com/docs/api.html>")
    /// * `to_local_path` - The target file's local path (e.g., "/output/example.com/about.html/index.html")
    /// * `output_dir` - The base output directory for all crawled files
    ///
    /// # Returns
    /// Relative path from source to target (e.g., "../../about.html/index.html")
    fn calculate_relative_path_sync(
        from_url: &str,
        to_local_path: &str,
        output_dir: &str,
    ) -> Result<String> {
        // Parse the source URL and determine its local path
        let parsed_url = Url::parse(from_url)?;
        let host = parsed_url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("URL has no host: {from_url}"))?;

        let url_path = if parsed_url.path() == "/" {
            std::path::PathBuf::new()
        } else {
            std::path::PathBuf::from(parsed_url.path().trim_start_matches('/'))
        };

        // Construct the local path for the source URL (same logic as get_mirror_path)
        let from_local = std::path::PathBuf::from(output_dir)
            .join(host)
            .join(url_path)
            .join("index.html");

        // Calculate relative path using pathdiff
        let from_path = Path::new(&from_local);
        let to_path = Path::new(to_local_path);

        let from_dir = from_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Source path has no parent directory"))?;

        let relative = pathdiff::diff_paths(to_path, from_dir).ok_or_else(|| {
            anyhow::anyhow!(
                "Could not calculate relative path from {} to {}",
                from_local.display(),
                to_local_path
            )
        })?;

        Ok(relative.to_string_lossy().to_string())
    }

    /// Get a map of all registered URLs
    pub async fn get_url_map(&self) -> HashMap<String, String> {
        let state = self.state.lock().await;
        // Convert LruCache to HashMap for external API
        state
            .url_to_local
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Rewrite links using data attributes (more efficient approach)
    pub async fn rewrite_links_from_data_attrs(&self, html: String) -> Result<String> {
        let state = self.state.clone();
        let output_dir = self.output_dir.clone();

        let mut output = Vec::new();

        // Clone for closure capture
        let state_for_closure = state.clone();
        let output_dir_for_closure = output_dir.clone();

        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!("a[data-absolute-href]", move |el| {
                    let absolute_href = match el.get_attribute("data-absolute-href") {
                        Some(h) => h,
                        None => return Ok(()),
                    };

                    // Check if we have a local version
                    let local_path = {
                        match state_for_closure.try_lock() {
                            Ok(mut state_guard) => {
                                // LruCache::get needs mutable access to update LRU order
                                state_guard.url_to_local.get(&absolute_href).cloned()
                            }
                            Err(_) => None, // Lock contended, skip this link
                        }
                    };

                    if let Some(local_path) = local_path
                        && let Some(current_url) = el.get_attribute("data-crawler-current-url")
                    {
                        // Calculate relative path
                        match Self::calculate_relative_path_sync(
                            &current_url,
                            &local_path,
                            &output_dir_for_closure,
                        ) {
                            Ok(relative_path) => {
                                el.set_attribute("href", &relative_path)?;
                            }
                            Err(e) => {
                                if let Some(href) = el.get_attribute("data-original-href") {
                                    log::warn!("Failed to calculate relative path for {href}: {e}");
                                }
                            }
                        }
                    }

                    // Clean up data attributes
                    el.remove_attribute("data-crawler-id");
                    el.remove_attribute("data-original-href");
                    el.remove_attribute("data-absolute-href");
                    el.remove_attribute("data-crawler-current-url");

                    Ok(())
                })],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );

        rewriter
            .write(html.as_bytes())
            .map_err(|e| anyhow::anyhow!("HtmlRewriter error: {e}"))?;
        rewriter
            .end()
            .map_err(|e| anyhow::anyhow!("HtmlRewriter end error: {e}"))?;

        String::from_utf8(output)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in rewritten HTML: {e}"))
    }

    /// Remove crawler-specific data attributes from final HTML
    #[must_use]
    pub fn remove_crawler_data_attrs(html: &str) -> String {
        // Remove our tracking attributes using regex - if any regex fails to compile,
        // just return the original HTML since these are optional cleanup operations
        let re1 = match Regex::new(r#" data-crawler-id="[^"]*""#) {
            Ok(re) => re,
            Err(_) => return html.to_string(),
        };
        let re2 = match Regex::new(r#" data-original-href="[^"]*""#) {
            Ok(re) => re,
            Err(_) => return html.to_string(),
        };
        let re3 = match Regex::new(r#" data-absolute-href="[^"]*""#) {
            Ok(re) => re,
            Err(_) => return html.to_string(),
        };
        let re4 = match Regex::new(r#" data-crawler-current-url="[^"]*""#) {
            Ok(re) => re,
            Err(_) => return html.to_string(),
        };

        let result = re1.replace_all(html, "");
        let result = re2.replace_all(&result, "");
        let result = re3.replace_all(&result, "");
        let result = re4.replace_all(&result, "");

        result.to_string()
    }
}
