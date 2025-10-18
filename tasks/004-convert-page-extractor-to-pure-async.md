# Task 004: Convert page_extractor Functions to Pure Async

## Status
NOT STARTED

## Dependencies
- Task 001 (runtime) must be complete
- Task 003 (content_saver) should be complete for html saving functions

## Core Objective

Convert all page_extractor callback-based functions to pure async, eliminating the `spawn_async` + callback + oneshot channel pattern in favor of direct async/await. This simplifies the code, reduces allocations, and makes the control flow more obvious.

**Pattern Transformation:**
```rust
// OLD (callback-based)
pub fn extract_metadata(
    page: Page,
    on_result: impl FnOnce(Result<PageMetadata>) + Send + 'static,
) -> AsyncTask<()>

// NEW (pure async)
pub async fn extract_metadata(
    page: Page,
) -> Result<PageMetadata>
```

## Architecture Context

The page_extractor module currently uses the callback pattern defined in [`runtime/async_task.rs`](../packages/citescrape/src/runtime/async_task.rs). This pattern was designed as a transition mechanism and is being phased out in favor of pure async/await.

**Current Pattern (deprecated):**
1. Function returns `AsyncTask<()>`
2. Takes `on_result` callback parameter
3. Wraps logic in `spawn_async(async move { ... on_result(result) })`
4. Callers create oneshot channels and await receivers

**New Pattern (target):**
1. Function is `async fn` returning `Result<T>`
2. No callback parameter
3. Direct async/await logic
4. Callers simply `.await` the function

**Relevant Files:**
- [`page_extractor/page_data.rs`](../packages/citescrape/src/page_extractor/page_data.rs) - Main orchestrator
- [`page_extractor/extractors.rs`](../packages/citescrape/src/page_extractor/extractors.rs) - Helper functions
- [`utils/url_utils.rs`](../packages/citescrape/src/utils/url_utils.rs) - Utility functions
- [`crawl_engine/core.rs`](../packages/citescrape/src/crawl_engine/core.rs) - Primary call site

## Files to Modify

### 1. [`packages/citescrape/src/page_extractor/extractors.rs`](../packages/citescrape/src/page_extractor/extractors.rs)

This file contains 7 callback-based extraction functions. All follow the same pattern.

#### Function: `extract_metadata` (Lines 16-40)

**Current Signature:**
```rust
pub fn extract_metadata(
    page: Page,
    on_result: impl FnOnce(Result<PageMetadata>) + Send + 'static,
) -> AsyncTask<()>
```

**Current Implementation Pattern:**
```rust
spawn_async(async move {
    let result = async {
        let js_result = page.evaluate(METADATA_SCRIPT).await.context("...")?;
        let metadata: PageMetadata = match js_result.into_value() {
            Ok(value) => serde_json::from_value(value).context("...")?,
            Err(e) => return Err(anyhow::anyhow!("...")),
        };
        Ok(metadata)
    }.await;
    on_result(result);
})
```

**NEW Signature:**
```rust
pub async fn extract_metadata(page: Page) -> Result<PageMetadata>
```

**NEW Implementation:**
```rust
pub async fn extract_metadata(page: Page) -> Result<PageMetadata> {
    let js_result = page
        .evaluate(METADATA_SCRIPT)
        .await
        .context("Failed to execute metadata extraction script")?;
    
    let metadata: PageMetadata = match js_result.into_value() {
        Ok(value) => serde_json::from_value(value)
            .context("Failed to parse metadata from JS result")?,
        Err(e) => return Err(anyhow::anyhow!("Failed to get metadata value: {}", e)),
    };
    
    Ok(metadata)
}
```

**Key Changes:**
- Remove `spawn_async` wrapper
- Remove `on_result` parameter and `AsyncTask<()>` return type
- Remove inner `async { ... }.await` block
- Return `Result<PageMetadata>` directly
- Keep all the extraction logic unchanged

#### Functions to Convert (same pattern):

1. **`extract_metadata`** (Line 16) → `async fn(...) -> Result<PageMetadata>`
2. **`extract_resources`** (Line 42) → `async fn(...) -> Result<ResourceInfo>`
3. **`extract_timing_info`** (Line 78) → `async fn(...) -> Result<TimingInfo>`
4. **`extract_security_info`** (Line 104) → `async fn(...) -> Result<SecurityInfo>`
5. **`extract_interactive_elements`** (Line 130) → `async fn(...) -> Result<Vec<InteractiveElement>>`
6. **`extract_links`** (Line 160) → `async fn(...) -> Result<Vec<CrawlLink>>`
7. **`capture_screenshot`** (Line 185) → `async fn(...) -> Result<()>`

All follow the identical transformation pattern - just remove the wrapper and callback machinery.

#### Special Case: `capture_screenshot` (Lines 185-254)

This function has additional complexity because it calls other callback-based functions (`get_mirror_path` and `save_compressed_file`). 

**Current Pattern (Lines 196-220):**
```rust
// Uses oneshot channel to await get_mirror_path
let (path_tx, path_rx) = tokio::sync::oneshot::channel();
let _path_task = crate::utils::get_mirror_path(&url, &output_dir, "index.png", move |result| {
    let _ = path_tx.send(result);
});
let path_result = path_rx.await.map_err(|_| anyhow::anyhow!("..."))?;
let path = path_result?;

// Uses oneshot channel to await save_compressed_file
let (save_tx, save_rx) = tokio::sync::oneshot::channel();
let save_guard = crate::content_saver::save_compressed_file(
    screenshot_data,
    &path,
    "image/png",
    move |metadata_result| {
        let _ = save_tx.send(metadata_result);
    }
);
let result = match save_rx.await { ... };
```

**NEW Pattern (after conversion):**
```rust
pub async fn capture_screenshot(
    page: Page,
    url: &str,
    output_dir: &std::path::Path,
) -> Result<()> {
    // Get mirror path (now pure async after url_utils conversion)
    let path = crate::utils::get_mirror_path(url, output_dir, "index.png").await?;
    
    std::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    )?;

    let params = CaptureScreenshotParams {
        quality: Some(100),
        format: Some(CaptureScreenshotFormat::Png),
        capture_beyond_viewport: Some(true),
        ..Default::default()
    };

    let screenshot_data = page.screenshot(params).await
        .map_err(|e| anyhow::anyhow!("Failed to capture screenshot: {}", e))?;

    // If save_compressed_file is still callback-based, use this pattern:
    let (save_tx, save_rx) = tokio::sync::oneshot::channel();
    let _save_guard = crate::content_saver::save_compressed_file(
        screenshot_data,
        &path,
        "image/png",
        move |metadata_result| {
            let _ = save_tx.send(metadata_result);
        }
    );
    
    // Wait for save to complete
    match save_rx.await {
        Ok(Ok(_metadata)) => {
            log::info!("Screenshot captured and saved successfully");
            Ok(())
        }
        Ok(Err(e)) => Err(anyhow::anyhow!("Failed to save screenshot: {}", e)),
        Err(_) => Err(anyhow::anyhow!("Screenshot save task was cancelled")),
    }
}
```

**Note:** The above assumes `save_compressed_file` is still callback-based (Task 003 dependency). If Task 003 converted it to pure async, simplify to:
```rust
crate::content_saver::save_compressed_file(screenshot_data, &path, "image/png").await?;
```

### 2. [`packages/citescrape/src/page_extractor/page_data.rs`](../packages/citescrape/src/page_extractor/page_data.rs)

#### Function: `extract_page_data` (Lines 195-392)

This is the most complex conversion because it orchestrates 7 parallel extraction tasks.

**Current Signature (Line 195):**
```rust
pub fn extract_page_data(
    page: Page,
    url: String,
    config: ExtractPageDataConfig,
    on_result: impl FnOnce(Result<super::schema::PageData>) + Send + 'static,
) -> AsyncTask<()>
```

**NEW Signature:**
```rust
pub async fn extract_page_data(
    page: Page,
    url: String,
    config: ExtractPageDataConfig,
) -> Result<super::schema::PageData>
```

#### Current Parallel Execution Pattern (Lines 206-272)

**OLD Pattern:**
```rust
// Create 7 oneshot channels
let (metadata_tx, metadata_rx) = tokio::sync::oneshot::channel();
let (resources_tx, resources_rx) = tokio::sync::oneshot::channel();
let (timing_tx, timing_rx) = tokio::sync::oneshot::channel();
let (security_tx, security_rx) = tokio::sync::oneshot::channel();
let (title_tx, title_rx) = tokio::sync::oneshot::channel();
let (interactive_tx, interactive_rx) = tokio::sync::oneshot::channel();
let (links_tx, links_rx) = tokio::sync::oneshot::channel();

// Launch 7 tasks with callbacks
let page_clone = page.clone();
let _metadata_task = extract_metadata(page_clone, move |result| {
    on_result!(result, metadata_tx, "Failed to extract metadata");
});

let page_clone = page.clone();
let _resources_task = extract_resources(page_clone, move |result| {
    on_result!(result, resources_tx, "Failed to extract resources");
});

// ... 5 more similar patterns ...

// Await all channels
let (metadata_result, resources_result, ...) = tokio::join!(
    async { metadata_rx.await.map_err(|_| anyhow::anyhow!("...")) },
    async { resources_rx.await.map_err(|_| anyhow::anyhow!("...")) },
    // ... more receivers
);

let metadata = metadata_result?;
let resources = resources_result?;
// ... unwrap all results
```

**NEW Pattern:**
```rust
// Launch all extraction tasks in parallel using tokio::spawn
let metadata_handle = tokio::spawn({
    let page = page.clone();
    async move { extract_metadata(page).await }
});

let resources_handle = tokio::spawn({
    let page = page.clone();
    async move { extract_resources(page).await }
});

let timing_handle = tokio::spawn({
    let page = page.clone();
    async move { extract_timing_info(page).await }
});

let security_handle = tokio::spawn({
    let page = page.clone();
    async move { extract_security_info(page).await }
});

let title_handle = tokio::spawn({
    let page = page.clone();
    async move {
        let title_value = page
            .evaluate("document.title")
            .await
            .context("Failed to evaluate document.title")?
            .into_value()
            .map_err(|e| anyhow::anyhow!("Failed to get page title: {}", e))?;

        if let serde_json::Value::String(title) = title_value {
            Ok(title)
        } else {
            Ok(String::new())
        }
    }
});

let interactive_handle = tokio::spawn({
    let page = page.clone();
    async move { extract_interactive_elements(page).await }
});

let links_handle = tokio::spawn({
    let page = page.clone();
    async move { extract_links(page).await }
});

// Wait for all tasks with proper error handling
// Note: tokio::spawn returns JoinHandle, need to await and unwrap JoinError
let metadata = metadata_handle.await
    .map_err(|e| anyhow::anyhow!("Metadata task panicked: {}", e))??;
let resources = resources_handle.await
    .map_err(|e| anyhow::anyhow!("Resources task panicked: {}", e))??;
let timing = timing_handle.await
    .map_err(|e| anyhow::anyhow!("Timing task panicked: {}", e))??;
let security = security_handle.await
    .map_err(|e| anyhow::anyhow!("Security task panicked: {}", e))??;
let title = title_handle.await
    .map_err(|e| anyhow::anyhow!("Title task panicked: {}", e))??;
let interactive_elements_vec = interactive_handle.await
    .map_err(|e| anyhow::anyhow!("Interactive elements task panicked: {}", e))??;
let links = links_handle.await
    .map_err(|e| anyhow::anyhow!("Links task panicked: {}", e))??;
```

**Alternative with tokio::try_join! (cleaner):**
```rust
// Launch and await all tasks in one expression
let (metadata, resources, timing, security, title, interactive_elements_vec, links) = tokio::try_join!(
    async {
        let page = page.clone();
        extract_metadata(page).await
    },
    async {
        let page = page.clone();
        extract_resources(page).await
    },
    async {
        let page = page.clone();
        extract_timing_info(page).await
    },
    async {
        let page = page.clone();
        extract_security_info(page).await
    },
    async {
        let page = page.clone();
        let title_value = page
            .evaluate("document.title")
            .await
            .context("Failed to evaluate document.title")?
            .into_value()
            .map_err(|e| anyhow::anyhow!("Failed to get page title: {}", e))?;

        if let serde_json::Value::String(title) = title_value {
            Ok::<_, anyhow::Error>(title)
        } else {
            Ok(String::new())
        }
    },
    async {
        let page = page.clone();
        extract_interactive_elements(page).await
    },
    async {
        let page = page.clone();
        extract_links(page).await
    },
)?;
```

#### Link Rewriter Calls (Lines 289-305)

The link rewriter functions `mark_links_for_discovery` and `rewrite_links_from_data_attrs` are also callback-based and need similar oneshot channel handling:

**Current Pattern:**
```rust
let (link_tx, link_rx) = tokio::sync::oneshot::channel();
let _link_task = config.link_rewriter.mark_links_for_discovery(&content, &url, move |result| {
    crate::on_result!(result, link_tx, "Failed to mark links for discovery");
});
let content_with_data_attrs = link_rx.await
    .map_err(|_| anyhow::anyhow!("Failed to mark links for discovery"))?;
```

**Keep this pattern** (these functions are in link_rewriter module, not part of this task):
```rust
// These stay as-is because link_rewriter is not part of this task
let (link_tx, link_rx) = tokio::sync::oneshot::channel();
let _link_task = config.link_rewriter.mark_links_for_discovery(&content, &url, move |result| {
    crate::on_result!(result, link_tx, "Failed to mark links for discovery");
});
let content_with_data_attrs = link_rx.await
    .map_err(|_| anyhow::anyhow!("Failed to mark links for discovery"))?;

let (rewrite_tx, rewrite_rx) = tokio::sync::oneshot::channel();
let _rewrite_task = config.link_rewriter.rewrite_links_from_data_attrs(content_with_data_attrs, move |result| {
    crate::on_result!(result, rewrite_tx, "Failed to rewrite links");
});
let content_with_rewritten_links = rewrite_rx.await
    .map_err(|_| anyhow::anyhow!("Failed to rewrite links"))?;
```

#### Save HTML Content (Lines 360-380)

The `save_html_content_with_resources` call is callback-based (Task 003 dependency). Keep the fire-and-forget pattern:

```rust
// Keep as-is (Task 003 dependency)
if config.save_html {
    let url_for_registration = url.clone();
    let _task = content_saver::save_html_content_with_resources(
        &content_with_rewritten_links,
        url.clone(),
        config.output_dir.clone(),
        &resources,
        config.max_inline_image_size_bytes,
        config.crawl_rate_rps,
        move |result| {
            match result {
                Ok(()) => log::info!("HTML content saved successfully for: {}", url_for_registration),
                Err(e) => log::warn!("Failed to save HTML for {}: {}", url_for_registration, e),
            }
        }
    );
}
```

#### Complete NEW Implementation Structure

```rust
pub async fn extract_page_data(
    page: Page,
    url: String,
    config: ExtractPageDataConfig,
) -> Result<super::schema::PageData> {
    log::info!("Starting to extract page data for URL: {}", url);

    // 1. Launch all parallel extractions
    let (metadata, resources, timing, security, title, interactive_elements_vec, links) = 
        tokio::try_join!(/* ... as shown above ... */)?;

    // 2. Get HTML content
    let content = page.content().await
        .map_err(|e| anyhow::anyhow!("Failed to get page content: {}", e))?;

    // 3. Link rewriting (still callback-based, not part of this task)
    let (link_tx, link_rx) = tokio::sync::oneshot::channel();
    let _link_task = config.link_rewriter.mark_links_for_discovery(&content, &url, move |result| {
        crate::on_result!(result, link_tx, "Failed to mark links for discovery");
    });
    let content_with_data_attrs = link_rx.await
        .map_err(|_| anyhow::anyhow!("Failed to mark links for discovery"))?;

    let (rewrite_tx, rewrite_rx) = tokio::sync::oneshot::channel();
    let _rewrite_task = config.link_rewriter.rewrite_links_from_data_attrs(content_with_data_attrs, move |result| {
        crate::on_result!(result, rewrite_tx, "Failed to rewrite links");
    });
    let content_with_rewritten_links = rewrite_rx.await
        .map_err(|_| anyhow::anyhow!("Failed to rewrite links"))?;

    // 4. Convert interactive elements
    let interactive_elements = convert_interactive_elements(interactive_elements_vec);

    // 5. Register URL mapping
    let (path_tx, path_rx) = tokio::sync::oneshot::channel();
    let path_url = url.clone();
    let path_output = config.output_dir.clone();
    let path_task = crate::utils::get_mirror_path(
        &path_url,
        &path_output,
        "index.html",
        move |result| {
            let _ = path_tx.send(result);
        }
    );
    let _path_guard = crate::runtime::TaskGuard::new(path_task, "get_mirror_path_for_registration");

    let local_path_result = crate::content_saver::await_with_timeout(path_rx, 30, "mirror path for URL registration").await;
    let local_path_str = match local_path_result {
        Ok(Ok(path)) => path.to_string_lossy().to_string(),
        Ok(Err(e)) => {
            log::warn!("Failed to get mirror path for URL registration: {}", e);
            config.output_dir.join("index.html").to_string_lossy().to_string()
        }
        Err(e) => {
            log::warn!("Timeout getting mirror path for URL registration: {}", e);
            config.output_dir.join("index.html").to_string_lossy().to_string()
        }
    };

    config.link_rewriter.register_url(&url, &local_path_str).await;

    // 6. Save HTML if enabled (fire-and-forget, Task 003 dependency)
    if config.save_html {
        let url_for_registration = url.clone();
        let _task = content_saver::save_html_content_with_resources(
            &content_with_rewritten_links,
            url.clone(),
            config.output_dir.clone(),
            &resources,
            config.max_inline_image_size_bytes,
            config.crawl_rate_rps,
            move |result| {
                match result {
                    Ok(()) => log::info!("HTML content saved successfully for: {}", url_for_registration),
                    Err(e) => log::warn!("Failed to save HTML for {}: {}", url_for_registration, e),
                }
            }
        );
    }

    log::info!("Successfully extracted page data for URL: {}", url);
    
    // 7. Return structured data
    Ok(super::schema::PageData {
        url: url.to_string(),
        title,
        content: content_with_rewritten_links,
        metadata,
        interactive_elements,
        links,
        resources,
        timing,
        security,
        crawled_at: chrono::Utc::now(),
    })
}
```

**Note:** After Task 003 is complete, the `get_mirror_path`, `save_html_content_with_resources`, and link rewriter functions will also be pure async, further simplifying this function.

### 3. [`packages/citescrape/src/utils/url_utils.rs`](../packages/citescrape/src/utils/url_utils.rs)

Two utility functions need conversion:

#### Function: `get_mirror_path` (Lines 42-68)

**Current Signature:**
```rust
pub fn get_mirror_path(
    url: &str,
    output_dir: &Path,
    filename: &str,
    on_result: impl FnOnce(Result<PathBuf>) + Send + 'static,
) -> AsyncTask<()>
```

**NEW Signature:**
```rust
pub async fn get_mirror_path(
    url: &str,
    output_dir: &Path,
    filename: &str,
) -> Result<PathBuf>
```

**NEW Implementation:**
```rust
pub async fn get_mirror_path(
    url: &str,
    output_dir: &Path,
    filename: &str,
) -> Result<PathBuf> {
    let url = Url::parse(url)
        .map_err(|e| anyhow::anyhow!("Failed to parse URL: {}", e))?;
    
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
```

**Note:** This function doesn't actually need to be async (no await calls), but we keep it async for consistency and future-proofing. Alternatively, remove the `async` and make it a pure sync function.

**Better approach:** Since this is pure computation with no I/O, **make it synchronous**:
```rust
pub fn get_mirror_path(
    url: &str,
    output_dir: &Path,
    filename: &str,
) -> Result<PathBuf> {
    // Same implementation, no async/await
}
```

#### Function: `get_uri_from_path` (Lines 12-40)

**Same pattern as get_mirror_path** - convert from callback to either sync or async based on whether it has actual async operations.

**Current Signature:**
```rust
pub fn get_uri_from_path(
    path: &Path,
    output_dir: &Path,
    on_result: impl FnOnce(Result<String>) + Send + 'static,
) -> AsyncTask<()>
```

**NEW Signature (synchronous - no I/O):**
```rust
pub fn get_uri_from_path(
    path: &Path,
    output_dir: &Path,
) -> Result<String>
```

**NEW Implementation:**
```rust
pub fn get_uri_from_path(
    path: &Path,
    output_dir: &Path,
) -> Result<String> {
    let result = path
        .strip_prefix(output_dir)
        .map_err(|e| anyhow::anyhow!("Failed to strip prefix: {}", e))?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path encoding"))?
        .replace('\\', "/")
        .to_string();

    Ok(result)
}
```

**Important Decision:** Since both functions are pure computation, **make them synchronous** instead of async. This is simpler and more efficient.

## Call Sites to Update

### [`packages/citescrape/src/crawl_engine/core.rs`](../packages/citescrape/src/crawl_engine/core.rs)

Three main call sites need updating:

#### 1. `extract_page_data` Call (Lines 578-607)

**OLD Pattern:**
```rust
// Extract page data
let (data_tx, data_rx) = tokio::sync::oneshot::channel();
let extract_config = crate::page_extractor::page_data::ExtractPageDataConfig {
    output_dir: config.storage_dir.clone(),
    link_rewriter: link_rewriter.clone(),
    max_inline_image_size_bytes: config.max_inline_image_size_bytes,
    crawl_rate_rps: config.crawl_rate_rps,
    save_html: config.save_raw_html(),
};
let task = page_extractor::extract_page_data(
    page.clone(),
    item.url.clone(),
    extract_config,
    move |result| {
        let _ = data_tx.send(result);
    }
);
let _data_guard = TaskGuard::new(task, "extract_page_data");

let page_data = match data_rx.await {
    Ok(Ok(data)) => data,
    Ok(Err(e)) => {
        warn!("Failed to extract page data for {}: {}", item.url, e);
        return Err(e);
    }
    Err(_) => {
        warn!("Data extraction channel closed for {}", item.url);
        return Err(anyhow::anyhow!("Data extraction channel closed"));
    }
};
```

**NEW Pattern:**
```rust
// Extract page data
let extract_config = crate::page_extractor::page_data::ExtractPageDataConfig {
    output_dir: config.storage_dir.clone(),
    link_rewriter: link_rewriter.clone(),
    max_inline_image_size_bytes: config.max_inline_image_size_bytes,
    crawl_rate_rps: config.crawl_rate_rps,
    save_html: config.save_raw_html(),
};

let page_data = page_extractor::extract_page_data(
    page.clone(),
    item.url.clone(),
    extract_config,
).await.map_err(|e| {
    warn!("Failed to extract page data for {}: {}", item.url, e);
    e
})?;
```

**Line Savings:** ~22 lines eliminated (from ~30 to ~8 lines)

#### 2. `capture_screenshot` Call (Lines 673-687)

**OLD Pattern:**
```rust
let mut screenshot_captured = false;
if config.save_screenshots() {
    let (screenshot_tx, screenshot_rx) = tokio::sync::oneshot::channel();
    let task = page_extractor::capture_screenshot(
        page.clone(),
        &item.url,
        config.storage_dir(),
        move |result| {
            let _ = screenshot_tx.send(result);
        }
    );
    let _screenshot_guard = TaskGuard::new(task, "capture_screenshot");
    
    match screenshot_rx.await {
        Ok(Ok(())) => {
            debug!("Screenshot saved for {}", item.url);
            screenshot_captured = true;
        }
        Ok(Err(e)) => warn!("Failed to save screenshot for {}: {}", item.url, e),
        Err(_) => warn!("Screenshot task cancelled for {}", item.url),
    }
}
```

**NEW Pattern:**
```rust
let mut screenshot_captured = false;
if config.save_screenshots() {
    match page_extractor::capture_screenshot(
        page.clone(),
        &item.url,
        config.storage_dir(),
    ).await {
        Ok(()) => {
            debug!("Screenshot saved for {}", item.url);
            screenshot_captured = true;
        }
        Err(e) => warn!("Failed to save screenshot for {}: {}", item.url, e),
    }
}
```

**Line Savings:** ~10 lines eliminated

#### 3. `get_mirror_path` Call (Lines 774-787)

**Note:** This already uses the sync version `get_mirror_path_sync`, so no changes needed if we keep url_utils functions synchronous.

**Current Code (Lines 774-787):**
```rust
let local_path = match crate::content_saver::get_mirror_path_sync(
    &item.url,
    &config.storage_dir,
    "index.md"
) {
    Ok(path) => path,
    Err(e) => {
        error!("Failed to compute local path for {}: {}", item.url, e);
        // ... fallback logic
    }
};
```

**If we made get_mirror_path async:**
```rust
let local_path = match crate::utils::get_mirror_path(
    &item.url,
    &config.storage_dir,
    "index.md"
).await {
    Ok(path) => path,
    Err(e) => {
        error!("Failed to compute local path for {}: {}", item.url, e);
        // ... fallback logic
    }
};
```

**Recommendation:** Keep url_utils functions **synchronous** since they're pure computation. The sync version is already exported and working.

## Transformation Patterns Reference

### Pattern 1: Basic Extractor Function

**Before:**
```rust
pub fn extract_something(
    page: Page,
    on_result: impl FnOnce(Result<DataType>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let js_result = page.evaluate(SCRIPT).await.context("...")?;
            let data: DataType = match js_result.into_value() {
                Ok(value) => serde_json::from_value(value).context("...")?,
                Err(e) => return Err(anyhow::anyhow!("...", e)),
            };
            Ok(data)
        }.await;
        on_result(result);
    })
}
```

**After:**
```rust
pub async fn extract_something(page: Page) -> Result<DataType> {
    let js_result = page.evaluate(SCRIPT).await.context("...")?;
    
    let data: DataType = match js_result.into_value() {
        Ok(value) => serde_json::from_value(value).context("...")?,
        Err(e) => return Err(anyhow::anyhow!("...", e)),
    };
    
    Ok(data)
}
```

### Pattern 2: Parallel Task Orchestration

**Before (oneshot channels):**
```rust
let (tx1, rx1) = tokio::sync::oneshot::channel();
let (tx2, rx2) = tokio::sync::oneshot::channel();

let _task1 = func1(args, move |result| { on_result!(result, tx1, "error"); });
let _task2 = func2(args, move |result| { on_result!(result, tx2, "error"); });

let (result1, result2) = tokio::join!(
    async { rx1.await.map_err(|_| anyhow::anyhow!("...")) },
    async { rx2.await.map_err(|_| anyhow::anyhow!("...")) },
);
let value1 = result1?;
let value2 = result2?;
```

**After (direct async/await):**
```rust
let (value1, value2) = tokio::try_join!(
    func1(args),
    func2(args),
)?;
```

### Pattern 3: Calling Callback Dependencies from Async

When calling functions that haven't been converted yet (Task 003 dependencies):

```rust
// Inside an async function, calling callback-based save_compressed_file
pub async fn my_async_function() -> Result<()> {
    // ... do work ...
    
    // Call callback-based function using oneshot channel
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _guard = save_compressed_file(data, path, mime, move |result| {
        let _ = tx.send(result);
    });
    
    // Await the result
    let metadata = rx.await
        .map_err(|_| anyhow::anyhow!("Channel closed"))??;
    
    Ok(())
}
```

### Pattern 4: Pure Computation (Don't Make It Async)

For functions that only do computation without I/O:

**Before:**
```rust
pub fn compute_path(
    url: &str,
    on_result: impl FnOnce(Result<PathBuf>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = (|| -> Result<PathBuf> {
            // Pure computation
            Ok(compute_something(url))
        })();
        on_result(result);
    })
}
```

**After (synchronous):**
```rust
pub fn compute_path(url: &str) -> Result<PathBuf> {
    // Pure computation, no async needed
    Ok(compute_something(url))
}
```

## Edge Cases and Special Considerations

### 1. TaskGuard Removal

The `TaskGuard` type is used to keep spawned tasks alive. After conversion:

**Before:**
```rust
let task = some_function(args, callback);
let _guard = TaskGuard::new(task, "task_name");
```

**After:**
```rust
// No guard needed - we're awaiting directly
let result = some_function(args).await?;
```

**Exception:** When calling callback-based dependencies, still need guards:
```rust
let _guard = callback_function(args, |result| { tx.send(result); });
```

### 2. Error Propagation

**Before (manual error handling):**
```rust
let result = match rx.await {
    Ok(Ok(value)) => value,
    Ok(Err(e)) => return Err(e),
    Err(_) => return Err(anyhow::anyhow!("Channel closed")),
};
```

**After (? operator):**
```rust
let result = some_function(args).await?;
```

### 3. Page Cloning for Parallel Tasks

When launching parallel tasks, each needs its own Page clone:

```rust
// Launch parallel tasks
let handle1 = tokio::spawn({
    let page = page.clone();  // Clone for this task
    async move { extract_metadata(page).await }
});

let handle2 = tokio::spawn({
    let page = page.clone();  // Clone for this task
    async move { extract_resources(page).await }
});
```

### 4. Mixed Callback/Async Dependencies

During transition, some dependencies may still be callback-based:

```rust
pub async fn my_function() -> Result<()> {
    // Call pure async function
    let data = extract_metadata(page).await?;
    
    // Call callback-based function (Task 003 dependency)
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _guard = save_data(data, move |result| { tx.send(result); });
    rx.await??;
    
    Ok(())
}
```

## Import Changes

After conversion, update imports in affected files:

**Remove:**
```rust
use crate::runtime::{spawn_async, AsyncTask};
use crate::on_result;
```

**Keep:**
```rust
use anyhow::{Context, Result};
use chromiumoxide::Page;
```

**Add (if using await_with_timeout helper):**
```rust
use crate::content_saver::await_with_timeout;
```

## Definition of Done

This task is complete when:

1. ✅ All 7 functions in `extractors.rs` are converted to `async fn` returning `Result<T>`
2. ✅ `extract_page_data` in `page_data.rs` is converted to `async fn` with direct async/await
3. ✅ `get_mirror_path` and `get_uri_from_path` in `url_utils.rs` are converted (synchronous preferred)
4. ✅ All call sites in `crawl_engine/core.rs` are updated to use direct `.await` calls
5. ✅ `cargo check --package citescrape` passes without errors
6. ✅ No callback parameters (`on_result`) remain in page_extractor module functions
7. ✅ No `spawn_async` wrappers in converted functions (except when calling callback dependencies)
8. ✅ No oneshot channels in converted functions (except when calling callback dependencies)
9. ✅ TaskGuard usage is eliminated from call sites (except for callback dependency calls)
10. ✅ The module still compiles and all type signatures match expected usage

**Verification Command:**
```bash
# Verify no callback patterns remain in page_extractor
grep -r "on_result: impl FnOnce" packages/citescrape/src/page_extractor/

# Should return no results (exit code 1)
# If it returns results, conversion is incomplete

# Verify compilation
cargo check --package citescrape
```

**Success Criteria:**
- `grep` command finds zero occurrences of callback pattern
- `cargo check` completes successfully
- Code is simpler and more readable than before

## Notes

- The actual extraction logic (JavaScript evaluation, DOM queries, etc.) remains unchanged
- This is purely a control flow refactoring, not a behavior change
- After Task 003 completes, revisit `extract_page_data` to simplify link rewriter and save operations
- Consider making `get_mirror_path` and `get_uri_from_path` synchronous since they're pure computation
- The `on_result!` macro can remain in the codebase for callback dependencies but shouldn't be used in converted functions
