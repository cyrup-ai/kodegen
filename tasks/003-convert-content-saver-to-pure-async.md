# Task 003: Convert content_saver Functions to Pure Async

## Status
NOT STARTED

## Dependencies
- Task 001 (runtime conversion) must be complete
- Task 002 (rate_limiter) should be complete
- Task 004 (url_utils conversion) must be complete before this task (provides async `get_mirror_path`)
- Task 005 (inline_css conversion) must be complete before this task (provides async inlining functions)

## Objective
Convert all content_saver callback-based functions to return `Future`s directly, eliminating callback hell and moving to clean async/await patterns.

## Core Transformation Pattern

### Pattern Overview

**OLD (Callback Hell):**
```rust
pub fn some_function(
    data: SomeType,
    on_result: impl FnOnce(Result<Output>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            // nested async work
        }.await;
        on_result(result);  // ← Callback invocation
    })
}
```

**NEW (Pure Async):**
```rust
pub async fn some_function(
    data: SomeType,
) -> Result<Output> {
    // direct async work
    Ok(output)
}
```

### What Gets Removed
1. ✂️ `spawn_async()` wrapper
2. ✂️ `on_result` callback parameter
3. ✂️ `oneshot::channel()` for result passing between callbacks
4. ✂️ `TaskGuard` RAII pattern (no longer needed)
5. ✂️ `await_with_timeout()` helper (replace with direct `.await?`)
6. ✂️ `log_send_error()` helper (no channels to fail)
7. ✂️ Nested `async { }.await` blocks

### What Gets Changed
1. 🔄 Function signature: remove callback → no callback
2. 🔄 Return type: `AsyncTask<()>` or `TaskGuard<()>` → `Result<T>`
3. 🔄 Function modifier: `pub fn` → `pub async fn`
4. 🔄 Internal logic: flatten nested async to direct async code
5. 🔄 End: `on_result(result)` → `return result` or just `Ok(value)`

### What Gets Kept
1. ✅ `tokio::task::spawn_blocking()` for CPU-intensive work (JSON serialization, etc.)
2. ✅ `tokio::fs::*` async filesystem operations
3. ✅ `tokio::join!()` for concurrent operations
4. ✅ Error handling with `?` operator and `anyhow::Error`
5. ✅ Logging with `log::info!`, `log::warn!`, etc.

## Files to Convert

### 1. [compression.rs](../packages/citescrape/src/content_saver/compression.rs)

**Location:** `/packages/citescrape/src/content_saver/compression.rs`

**Function:** `save_compressed_file()` (lines 27-116)

**BEFORE:**
```rust
pub fn save_compressed_file(
    content: Vec<u8>,
    path: &Path,
    content_type: &str,
    on_result: impl FnOnce(Result<CacheMetadata, anyhow::Error>) + Send + 'static,
) -> TaskGuard<()> {
    let path = path.to_path_buf();
    let content_type = content_type.to_string();
    
    let task = spawn_async(async move {
        let result: Result<CacheMetadata> = async {
            // Calculate XXHash for etag
            let hash = xxhash_rust::xxh3::xxh3_64(&content);
            let etag = format!("\"{:x}\"", hash);
            
            // ... rest of compression logic ...
            
            Ok(metadata)
        }.await;
        
        on_result(result);
    });
    
    TaskGuard::new(task, "save_compressed_file")
}
```

**AFTER:**
```rust
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
    
    let filename_str = path.file_name()
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
                .write(file, Compression::new(3));
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
            .write(file, Compression::new(3));
        gz.write_all(&content)?;
        gz.finish()?;
    }

    Ok(metadata)
}
```

**Key Changes:**
- Removed `on_result` callback parameter
- Changed return type from `TaskGuard<()>` to `Result<CacheMetadata>`
- Changed `pub fn` to `pub async fn`
- Removed `spawn_async` wrapper
- Removed nested `async { }.await` block
- Removed `on_result(result)` call
- Directly return `Ok(metadata)` at the end
- Kept `spawn_blocking` for large file compression (CPU-intensive)

---

### 2. [json_saver.rs](../packages/citescrape/src/content_saver/json_saver.rs)

**Location:** `/packages/citescrape/src/content_saver/json_saver.rs`

#### Function 1: `save_json_data()` (lines 10-72)

**BEFORE:**
```rust
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
```

**AFTER:**
```rust
pub async fn save_json_data(
    data: serde_json::Value, 
    url: String, 
    output_dir: std::path::PathBuf,
) -> Result<()> {
    // get_mirror_path is now async (from Task 004)
    let path = get_mirror_path(&url, &output_dir, "index.json").await?;
    
    // JSON serialization (keep spawn_blocking - CPU intensive)
    let json_str = tokio::task::spawn_blocking(move || {
        serde_json::to_string_pretty(&data)
    })
    .await
    .map_err(|e| anyhow::anyhow!("JSON serialization task panicked: {}", e))??;
    
    // Create directory
    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    ).await?;
    
    // save_compressed_file is now async (converted in this task)
    let _metadata = save_compressed_file(
        json_str.into_bytes(),
        &path,
        "application/json",
    ).await?;
    
    Ok(())
}
```

**Key Changes:**
- Removed `on_result` callback parameter
- Changed return type from `AsyncTask<()>` to `Result<()>`
- Changed `pub fn` to `pub async fn`
- Removed `spawn_async` wrapper
- Removed ALL oneshot channels (`json_path_tx/rx`, `json_compress_tx/rx`)
- Removed ALL `TaskGuard` instances
- Removed `await_with_timeout` - replaced with direct `.await?`
- Removed `log_send_error` calls (no channels to fail)
- Call `get_mirror_path` directly with `.await?` (assumes Task 004 complete)
- Call `save_compressed_file` directly with `.await?`
- Kept `spawn_blocking` for JSON serialization (CPU-intensive)
- Removed `tokio::join!` - no longer needed since operations are sequential

#### Function 2: `save_page_data()` (lines 75-132)

**Apply identical transformation pattern as `save_json_data()`**

**AFTER:**
```rust
pub async fn save_page_data(
    page_data: std::sync::Arc<crate::page_extractor::schema::PageData>,
    url: String,
    output_dir: std::path::PathBuf,
) -> Result<()> {
    // get_mirror_path is now async (from Task 004)
    let path = get_mirror_path(&url, &output_dir, "index.json").await?;
    
    // Serialize PageData on blocking thread pool
    let json_content = tokio::task::spawn_blocking(move || {
        serde_json::to_string_pretty(&*page_data)
    })
    .await
    .map_err(|e| anyhow::anyhow!("PageData serialization task panicked: {}", e))??;
    
    // save_compressed_file is now async (converted in this task)
    let _metadata = save_compressed_file(
        json_content.into_bytes(), 
        &path, 
        "application/json",
    ).await?;
    
    Ok(())
}
```

---

### 3. [markdown_saver.rs](../packages/citescrape/src/content_saver/markdown_saver.rs)

**Location:** `/packages/citescrape/src/content_saver/markdown_saver.rs`

**Function:** `save_markdown_content()` (lines 27-107)

**BEFORE:**
```rust
pub fn save_markdown_content(
    markdown_content: String,
    url: String,
    output_dir: std::path::PathBuf,
    priority: MessagePriority,
    indexing_sender: Option<Arc<IndexingSender>>,
    on_result: impl FnOnce(Result<()>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
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
                // ... indexing logic ...
            }
            
            log::info!("Saved markdown for {} to {} (etag: {})", 
                url, path.display(), metadata.etag);
            
            Ok(())
        }.await;
        
        on_result(result);
    })
}
```

**AFTER:**
```rust
pub async fn save_markdown_content(
    markdown_content: String,
    url: String,
    output_dir: std::path::PathBuf,
    priority: MessagePriority,
    indexing_sender: Option<Arc<IndexingSender>>,
) -> Result<()> {
    // get_mirror_path is now async (from Task 004)
    let path = get_mirror_path(&url, &output_dir, "index.md").await?;
    
    // Ensure parent directory exists
    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    ).await?;
    
    // save_compressed_file is now async (converted in this task)
    let metadata = save_compressed_file(
        markdown_content.into_bytes(),
        &path,
        "text/markdown",
    ).await?;
    
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
```

**Key Changes:**
- Same pattern as json_saver functions
- Removed all oneshot channels and TaskGuards
- Call dependencies directly with `.await?`
- Keep indexing logic (it uses callbacks internally, which is fine)

---

### 4. [html_saver.rs](../packages/citescrape/src/content_saver/html_saver.rs)

**Location:** `/packages/citescrape/src/content_saver/html_saver.rs`

#### Function 1: `save_html_content()` (lines 12-107)

**AFTER:**
```rust
pub async fn save_html_content(
    html_content: String, 
    url: String, 
    output_dir: std::path::PathBuf,
    max_inline_image_size_bytes: Option<usize>,
    rate_rps: Option<f64>,
) -> Result<()> {
    // Wrap html_content in Arc to avoid expensive clones
    let html_arc = Arc::new(html_content);
    
    let config = crate::inline_css::InlineConfig::default();
    
    // Start both operations in parallel using tokio::join!
    let (inline_result, path_result) = tokio::join!(
        // inline_all_resources is now async (from Task 005)
        crate::inline_css::inline_all_resources(
            (*html_arc).clone(), 
            url.clone(), 
            &config, 
            max_inline_image_size_bytes, 
            rate_rps
        ),
        // get_mirror_path is now async (from Task 004)
        get_mirror_path(&url, &output_dir, "index.html")
    );
    
    let inlined_html = match inline_result {
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
    };
    
    let path = path_result?;
    
    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    ).await?;
    
    // save_compressed_file is now async (converted in this task)
    let _metadata = save_compressed_file(
        inlined_html.into_bytes(), 
        &path, 
        "text/html",
    ).await?;
    
    Ok(())
}
```

**Key Changes:**
- Removed callback parameter and `spawn_async` wrapper
- Removed ALL oneshot channels
- Use `tokio::join!` for parallel operations
- Call `inline_all_resources` directly (assumes Task 005 complete)
- Call `get_mirror_path` directly (assumes Task 004 complete)
- Call `save_compressed_file` directly
- Simplified error handling - no need for channel error tracking

#### Function 2: `save_html_content_with_resources()` (lines 110-221)

**Apply identical transformation pattern as `save_html_content()`**

**AFTER:**
```rust
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
    
    // Wrap html_content in Arc to avoid expensive clones
    let html_arc = Arc::new(html_content);
    
    let config = crate::inline_css::InlineConfig::default();
    
    // Start both operations in parallel
    let (inline_result, path_result) = tokio::join!(
        // inline_resources_from_info is now async (from Task 005)
        crate::inline_css::inline_resources_from_info(
            (*html_arc).clone(), 
            url.clone(), 
            &config, 
            resources.clone(), 
            max_inline_image_size_bytes, 
            rate_rps
        ),
        // get_mirror_path is now async (from Task 004)
        get_mirror_path(&url, &output_dir, "index.html")
    );
    
    let inlined_html = match inline_result {
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
    };
    
    let path = path_result?;
    
    tokio::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| anyhow::anyhow!("Path has no parent directory"))?,
    ).await?;
    
    // save_compressed_file is now async (converted in this task)
    let _metadata = save_compressed_file(
        inlined_html.into_bytes(), 
        &path, 
        "text/html",
    ).await?;
    
    Ok(())
}
```

---

### 5. [markdown_converter/mod.rs](../packages/citescrape/src/content_saver/markdown_converter/mod.rs)

**Location:** `/packages/citescrape/src/content_saver/markdown_converter/mod.rs`

**Function:** `convert_html_to_markdown_async()` (lines 307-326)

**BEFORE:**
```rust
pub fn convert_html_to_markdown_async(
    html: &str,
    options: &ConversionOptions,
    on_result: impl FnOnce(Result<String>) + Send + 'static,
) -> AsyncTask<()> {
    let html = html.to_string();
    let options = options.clone();

    spawn_async(async move {
        // Execute the entire pipeline synchronously within the async task
        // This is appropriate because the pipeline is CPU-bound, not I/O-bound
        let result = convert_html_to_markdown(&html, &options);
        on_result(result);
    })
}
```

**AFTER:**
```rust
pub async fn convert_html_to_markdown_async(
    html: &str,
    options: &ConversionOptions,
) -> Result<String> {
    let html = html.to_string();
    let options = options.clone();
    
    // Execute on blocking thread pool since HTML parsing/conversion is CPU-intensive
    tokio::task::spawn_blocking(move || {
        convert_html_to_markdown(&html, &options)
    })
    .await
    .map_err(|e| anyhow::anyhow!("HTML to markdown conversion task panicked: {}", e))?
}
```

**Key Changes:**
- Removed callback parameter
- Changed return type from `AsyncTask<()>` to `Result<String>`
- Changed `pub fn` to `pub async fn`
- Replaced `spawn_async` with `tokio::task::spawn_blocking` (proper pattern for CPU-bound work)
- This correctly offloads HTML parsing to the blocking thread pool
- Removed `on_result(result)` call

**Note:** The sync version `convert_html_to_markdown()` remains unchanged and is the actual implementation.

---

## Call Sites to Update

### [crawl_engine/core.rs](../packages/citescrape/src/crawl_engine/core.rs)

**Location:** `/packages/citescrape/src/crawl_engine/core.rs` (lines ~612-665)

#### Call Site 1: `convert_html_to_markdown_async` (lines ~612-630)

**BEFORE:**
```rust
let (markdown_tx, markdown_rx) = tokio::sync::oneshot::channel();
let conversion_options = ConversionOptions::default();
let page_data_content = page_data.content.clone();

let task = convert_html_to_markdown_async(
    &page_data.content,
    &conversion_options,
    move |result| {
        let final_markdown = match result {
            Ok(md) => md,
            Err(e) => {
                warn!("Markdown conversion failed: {}, using html2md fallback", e);
                html2md::parse_html(&page_data_content)
            }
        };
        let _ = markdown_tx.send(final_markdown);
    }
);
let _markdown_guard = TaskGuard::new(task, "markdown_conversion");

if let Ok(processed_markdown) = markdown_rx.await {
    // ... save logic ...
}
```

**AFTER:**
```rust
let conversion_options = ConversionOptions::default();
let page_data_content = page_data.content.clone();

let processed_markdown = match convert_html_to_markdown_async(
    &page_data.content,
    &conversion_options,
).await {
    Ok(md) => md,
    Err(e) => {
        warn!("Markdown conversion failed: {}, using html2md fallback", e);
        html2md::parse_html(&page_data_content)
    }
};

// Continue to save logic...
```

**Key Changes:**
- Removed oneshot channel (`markdown_tx`/`markdown_rx`)
- Removed callback
- Removed `TaskGuard`
- Direct `.await` on the async function
- Handle result inline with `match`

#### Call Site 2: `save_markdown_content` (lines ~631-649)

**BEFORE:**
```rust
let (save_tx, save_rx) = tokio::sync::oneshot::channel();
let task = content_saver::save_markdown_content(
    processed_markdown,
    item.url.clone(),
    config.storage_dir.clone(),
    crate::search::MessagePriority::Normal,
    indexing_sender.clone(),
    move |result| {
        let _ = save_tx.send(result);
    }
);
let _save_guard = TaskGuard::new(task, "save_markdown_content");

match save_rx.await {
    Ok(Ok(())) => debug!("Markdown saved for {}", item.url),
    Ok(Err(e)) => warn!("Failed to save markdown for {}: {}", item.url, e),
    Err(_) => warn!("Markdown save channel closed for {}", item.url),
}
```

**AFTER:**
```rust
match content_saver::save_markdown_content(
    processed_markdown,
    item.url.clone(),
    config.storage_dir.clone(),
    crate::search::MessagePriority::Normal,
    indexing_sender.clone(),
).await {
    Ok(()) => debug!("Markdown saved for {}", item.url),
    Err(e) => warn!("Failed to save markdown for {}: {}", item.url, e),
}
```

**Key Changes:**
- Removed oneshot channel (`save_tx`/`save_rx`)
- Removed callback
- Removed `TaskGuard`
- Direct `.await` with inline error handling
- Simpler match: `Ok(())` vs `Err(e)` instead of nested `Ok(Ok(()))`, `Ok(Err(e))`, `Err(_)`

#### Call Site 3: `save_page_data` (lines ~652-665)

**Apply identical transformation:**

**AFTER:**
```rust
match content_saver::save_page_data(
    Arc::new(page_data.clone()),
    item.url.clone(),
    config.storage_dir.clone(),
).await {
    Ok(()) => debug!("Page data saved for {}", item.url),
    Err(e) => warn!("Failed to save page data for {}: {}", item.url, e),
}
```

---

## Helper Functions to Remove

### From [mod.rs](../packages/citescrape/src/content_saver/mod.rs)

These helpers become obsolete after conversion:

#### 1. `await_with_timeout()` (lines ~42-56)

**Status:** DELETE after all conversions complete

This helper was used to wrap oneshot channel receives with timeout protection. With pure async, we just use `.await?` directly and rely on tokio's task cancellation.

**If timeout is truly needed**, use `tokio::time::timeout` directly at call sites:
```rust
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(30), 
    some_async_function()
).await??;
```

#### 2. `log_send_error()` (lines ~73-93)

**Status:** DELETE after all conversions complete

This helper logged errors when oneshot channels failed to send. With no channels, this is no longer needed.

---

## Imports to Update

### In each modified file:

**REMOVE these imports:**
```rust
use crate::runtime::{spawn_async, AsyncTask, TaskGuard};
use super::{await_with_timeout, log_send_error};
use tokio::sync::oneshot;
```

**KEEP these imports:**
```rust
use anyhow::Result;
use tokio::task::spawn_blocking;  // For CPU-bound work
use tokio::fs;                    // For async file operations
use tokio::join;                  // For concurrent operations
```

**ADD if not present:**
```rust
use crate::utils::get_mirror_path;  // Now async from Task 004
use crate::inline_css::{inline_all_resources, inline_resources_from_info};  // Now async from Task 005
```

---

## Verification Steps

After converting each file:

1. **Syntax Check:**
   ```bash
   cargo check --package citescrape
   ```

2. **Verify No Callbacks Remain:**
   ```bash
   # Should return no results:
   rg "on_result.*FnOnce" packages/citescrape/src/content_saver/
   ```

3. **Verify All Calls Use .await:**
   ```bash
   # Check that new async functions are called with .await:
   rg "save_compressed_file\(" packages/citescrape/src/content_saver/
   # Each call should have .await
   ```

4. **Verify No TaskGuards:**
   ```bash
   # Should only find TaskGuard definition, no usage:
   rg "TaskGuard::new" packages/citescrape/src/content_saver/
   ```

5. **Verify No Oneshot Channels:**
   ```bash
   # Should find no new oneshot channel creation:
   rg "oneshot::channel" packages/citescrape/src/content_saver/
   ```

---

## Definition of Done

✅ All 6 functions converted to pure async:
- `save_compressed_file` in compression.rs
- `save_json_data` in json_saver.rs
- `save_page_data` in json_saver.rs
- `save_markdown_content` in markdown_saver.rs
- `save_html_content` in html_saver.rs
- `save_html_content_with_resources` in html_saver.rs
- `convert_html_to_markdown_async` in markdown_converter/mod.rs

✅ All call sites in crawl_engine/core.rs updated to use direct `.await`

✅ `cargo check --package citescrape` passes without errors

✅ No callback parameters remain in converted functions

✅ No `TaskGuard` usage in content_saver module

✅ No oneshot channels in content_saver module

✅ Helper functions `await_with_timeout` and `log_send_error` removed from mod.rs

---

## Implementation Order

**Recommended sequence:**

1. **First:** Convert `save_compressed_file` (compression.rs)
   - It has no dependencies on other content_saver functions
   - Other functions depend on it

2. **Second:** Convert json_saver.rs functions
   - Depends on `save_compressed_file` (now async)
   - Depends on `get_mirror_path` (async from Task 004)

3. **Third:** Convert `save_markdown_content` (markdown_saver.rs)
   - Depends on `save_compressed_file` (now async)
   - Depends on `get_mirror_path` (async from Task 004)

4. **Fourth:** Convert html_saver.rs functions
   - Depends on `save_compressed_file` (now async)
   - Depends on `get_mirror_path` (async from Task 004)
   - Depends on `inline_all_resources` and `inline_resources_from_info` (async from Task 005)

5. **Fifth:** Convert `convert_html_to_markdown_async` (markdown_converter/mod.rs)
   - Independent, can be done anytime

6. **Finally:** Update call sites in crawl_engine/core.rs
   - Update all three call sites
   - Remove helper functions from mod.rs

7. **Cleanup:** Run verification steps and `cargo check`

---

## Notes

- This task assumes Task 004 (url_utils) and Task 005 (inline_css) are complete
- The core pattern is: remove callbacks, flatten async, use direct `.await?`
- Keep `spawn_blocking` for CPU-intensive work (JSON serialization, HTML parsing)
- The result is much cleaner, more readable async/await code
- Error handling becomes simpler with the `?` operator
- No more callback hell or oneshot channel bridging
