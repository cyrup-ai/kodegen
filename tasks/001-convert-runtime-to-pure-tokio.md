# Task 001: Convert Runtime Module to Pure Tokio

## Status
NOT STARTED

## Objective
Eliminate callback-based async patterns and convert to pure tokio async/await. Remove legacy runtime abstractions that add complexity without benefit.

## Background & Problem Analysis

### Current Architecture Issues

The codebase uses a **callback-based async pattern** that wraps tokio's native async/await:

```rust
// CALLBACK PATTERN (current - WRONG)
pub fn extract_metadata(
    page: Page,
    on_result: impl FnOnce(Result<PageMetadata>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            // actual work here
        }.await;
        on_result(result);  // Call callback with result
    })
}

// Usage requires ceremony:
let (tx, rx) = tokio::sync::oneshot::channel();
let _task = extract_metadata(page, move |result| {
    on_result!(result, tx, "Failed to extract metadata");
});
let metadata = rx.await?;
```

**Why this is bad:**
1. Requires oneshot channels for every call
2. Uses `on_result!` macro which doesn't propagate errors (just logs!)
3. Adds indirection and complexity
4. Harder to compose with tokio::join!, tokio::select!, etc.
5. chromiumoxide library is ALREADY pure tokio async - we're wrapping it unnecessarily!

### What Pure Tokio Looks Like

```rust
// PURE ASYNC (correct - what we want)
pub async fn extract_metadata(page: Page) -> Result<PageMetadata> {
    let js_result = page.evaluate(METADATA_SCRIPT).await
        .context("Failed to execute metadata extraction script")?;
    
    let metadata: PageMetadata = match js_result.into_value() {
        Ok(value) => serde_json::from_value(value)
            .context("Failed to parse metadata from JS result")?,
        Err(e) => return Err(anyhow::anyhow!("Failed to get metadata value: {}", e)),
    };
    
    Ok(metadata)
}

// Usage is simple and composable:
let metadata = extract_metadata(page.clone()).await?;

// Easy parallelism:
let (metadata, resources, timing) = tokio::try_join!(
    extract_metadata(page.clone()),
    extract_resources(page.clone()),
    extract_timing_info(page.clone())
)?;
```

## Files Already Completed

✅ **DELETED**: `runtime/thread_pool.rs` - File doesn't exist (already removed)
✅ **DELETED**: `runtime/zero_alloc.rs` - File doesn't exist (already removed)
✅ **COMMENTED OUT**: Lines 10-11 in `runtime/mod.rs` - References to deleted modules

## Files to Modify

### 1. CLEANUP: [`/packages/citescrape/src/runtime/mod.rs`](../packages/citescrape/src/runtime/mod.rs)

**Current Issues:**
- Lines 77-100: `on_result!` and `on_unit_result!` macros (callback pattern support)
- Lines 102-130: `executor` module (no-op wrapper around tokio)

**Changes Required:**

#### Delete Lines 77-100 (callback macros)
```rust
// DELETE THIS ENTIRE SECTION:
#[macro_export]
macro_rules! on_result {
    ($result:expr, $tx:expr, $err_msg:expr) => {
        match $result {
            Ok(value) => { let _ = $tx.send(value); }
            Err(e) => { 
                log::error!("{}: {}", $err_msg, e);
                // BUG: doesn't send error! Receiver will hang forever!
            }
        }
    };
}

#[macro_export]
macro_rules! on_unit_result {
    // ... delete this too
}
```

#### Delete Lines 102-130 (executor module)
```rust
// DELETE THIS ENTIRE SECTION:
pub mod executor {
    // This is just a no-op wrapper around tokio
    // It provides zero value
}
```

**After cleanup, runtime/mod.rs should be:**
```rust
//! Pure tokio async runtime

pub mod async_stream;
pub mod async_task;
pub mod async_wrappers;
pub mod channel;

pub use async_stream::{AsyncStream, StreamSender, TrySendError};
pub use async_task::{spawn_async, spawn_stream, AsyncTask, TaskGuard, ready, pending, TaskError};
pub use async_wrappers::{AsyncJsonSave, BrowserAction, CrawlRequest};
pub use channel::*;

/// Create channel with optimal configuration
#[inline(always)]
pub fn create_channel<T>() -> (tokio::sync::mpsc::UnboundedSender<T>, tokio::sync::mpsc::UnboundedReceiver<T>) {
    tokio::sync::mpsc::unbounded_channel()
}
```

### 2. SIMPLIFY: [`/packages/citescrape/src/runtime/async_task.rs`](../packages/citescrape/src/runtime/async_task.rs)

**Current Issue:**
Line 48 calls `super::executor::register_waker(cx.waker().clone())` which is a no-op.

**Delete the no-op call:**
```rust
// BEFORE (lines 43-58):
Err(mpsc::error::TryRecvError::Empty) => {
    // Register waker for notification and double-check atomically
    super::executor::register_waker(cx.waker().clone());  // ← DELETE THIS LINE
    
    // Double-check pattern to avoid race conditions
    match rx.try_recv() {
        Ok(value) => Poll::Ready(Ok(value)),
        Err(mpsc::error::TryRecvError::Empty) => Poll::Pending,
        Err(mpsc::error::TryRecvError::Disconnected) => {
            Poll::Ready(Err(TaskError::Disconnected))
        }
    }
}

// AFTER (simplified):
Err(mpsc::error::TryRecvError::Empty) => {
    // Tokio runtime handles waker registration automatically
    // Double-check pattern to avoid race conditions
    match rx.try_recv() {
        Ok(value) => Poll::Ready(Ok(value)),
        Err(mpsc::error::TryRecvError::Empty) => Poll::Pending,
        Err(mpsc::error::TryRecvError::Disconnected) => {
            Poll::Ready(Err(TaskError::Disconnected))
        }
    }
}
```

**Note**: `async_task.rs` is otherwise fine. The `spawn_async` helper and `AsyncTask` type provide useful abstractions and can remain.

### 3. CONVERT: [`/packages/citescrape/src/page_extractor/extractors.rs`](../packages/citescrape/src/page_extractor/extractors.rs)

**Functions to convert (all follow same pattern):**
- `extract_metadata` (lines 15-37)
- `extract_resources` (lines 41-72)
- `extract_timing_info` (lines 76-94)
- `extract_security_info` (lines 98-116)
- `extract_interactive_elements` (lines 120-149)
- `extract_links` (lines 153-171)
- `capture_screenshot` (lines 174-253)

**Conversion Template:**

```rust
// BEFORE (callback-based):
#[inline]
pub fn extract_metadata(
    page: Page,
    on_result: impl FnOnce(Result<PageMetadata>) + Send + 'static,
) -> AsyncTask<()> {
    spawn_async(async move {
        let result = async {
            let js_result = page.evaluate(METADATA_SCRIPT).await
                .context("Failed to execute metadata extraction script")?;
            
            let metadata: PageMetadata = match js_result.into_value() {
                Ok(value) => serde_json::from_value(value)
                    .context("Failed to parse metadata from JS result")?,
                Err(e) => return Err(anyhow::anyhow!("Failed to get metadata value: {}", e)),
            };
            
            Ok(metadata)
        }.await;
        
        on_result(result);
    })
}

// AFTER (pure async):
#[inline]
pub async fn extract_metadata(page: Page) -> Result<PageMetadata> {
    let js_result = page.evaluate(METADATA_SCRIPT).await
        .context("Failed to execute metadata extraction script")?;
    
    let metadata: PageMetadata = match js_result.into_value() {
        Ok(value) => serde_json::from_value(value)
            .context("Failed to parse metadata from JS result")?,
        Err(e) => return Err(anyhow::anyhow!("Failed to get metadata value: {}", e)),
    };
    
    Ok(metadata)
}
```

**Key Changes for Each Function:**
1. Change signature: `pub fn` → `pub async fn`
2. Remove `on_result` parameter
3. Change return type: `AsyncTask<()>` → `Result<T>`
4. Remove `spawn_async` wrapper
5. Remove inner `let result = async { ... }.await;` nesting
6. Remove `on_result(result);` call
7. Directly return the result

### 4. CONVERT: [`/packages/citescrape/src/page_extractor/link_rewriter.rs`](../packages/citescrape/src/page_extractor/link_rewriter.rs)

**Functions to convert:**
- `mark_links_for_discovery` (lines 94-162)
- `rewrite_links` (lines 165-262)
- `calculate_relative_path` (lines 312-324)
- `url_to_local_path` (lines 328-352)
- `rewrite_links_from_data_attrs` (lines 360-437)

**Same conversion pattern as extractors.rs**

**Example:**
```rust
// BEFORE:
pub fn mark_links_for_discovery(
    &self, 
    html: &str, 
    current_url: &str,
    on_result: impl FnOnce(Result<String>) + Send + 'static,
) -> AsyncTask<()> {
    let html = html.to_string();
    let current_url = current_url.to_string();
    
    spawn_async(async move {
        let result = (|| -> Result<String> {
            // ... actual work ...
        })();
        
        on_result(result);
    })
}

// AFTER:
pub async fn mark_links_for_discovery(
    &self, 
    html: &str, 
    current_url: &str,
) -> Result<String> {
    let html = html.to_string();
    let current_url = current_url.to_string();
    
    // ... actual work directly ...
    // No spawn_async, no callback
}
```

### 5. CONVERT: [`/packages/citescrape/src/crawl_engine/page_enhancer.rs`](../packages/citescrape/src/crawl_engine/page_enhancer.rs)

**Function:** `enhance_page`

Same conversion pattern.

### 6. CONVERT: [`/packages/citescrape/src/crawl_engine/link_processor.rs`](../packages/citescrape/src/crawl_engine/link_processor.rs)

**Function:** `process_links_for_crawl`

Same conversion pattern.

### 7. CONVERT: [`/packages/citescrape/src/search/engine.rs`](../packages/citescrape/src/search/engine.rs)

**Functions to convert:**
- `new` (lines 30-87)
- `acquire_writer_with_retry` (lines 124-147)
- `acquire_writer` (lines 150-166)
- `commit_index` (lines 192-220)

Same conversion pattern.

### 8. UPDATE CALL SITES: [`/packages/citescrape/src/page_extractor/page_data.rs`](../packages/citescrape/src/page_extractor/page_data.rs)

This is the primary consumer of the callback pattern. Lines 200-392 show extensive use.

**Current Pattern (lines 216-233):**
```rust
// Pre-allocate all channels for parallel extraction
let (metadata_tx, metadata_rx) = tokio::sync::oneshot::channel();
let (resources_tx, resources_rx) = tokio::sync::oneshot::channel();
let (timing_tx, timing_rx) = tokio::sync::oneshot::channel();
let (security_tx, security_rx) = tokio::sync::oneshot::channel();
let (title_tx, title_rx) = tokio::sync::oneshot::channel();
let (interactive_tx, interactive_rx) = tokio::sync::oneshot::channel();
let (links_tx, links_rx) = tokio::sync::oneshot::channel::<Vec<super::schema::CrawlLink>>();

// Launch all extraction tasks in parallel
let page_clone = page.clone();
let _metadata_task = extract_metadata(page_clone, move |result| {
    on_result!(result, metadata_tx, "Failed to extract metadata");
});

let page_clone = page.clone();
let _resources_task = extract_resources(page_clone, move |result| {
    on_result!(result, resources_tx, "Failed to extract resources");
});

// ... more tasks ...

// Await all parallel extractions
let (metadata_result, resources_result, ...) = tokio::join!(
    async { metadata_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract metadata")) },
    async { resources_rx.await.map_err(|_| anyhow::anyhow!("Failed to extract resources")) },
    // ...
);
```

**Convert to Pure Async:**
```rust
// Launch all extractions in parallel with tokio::join!
let (metadata, resources, timing, security, title, interactive_elements_vec, links) = tokio::try_join!(
    extract_metadata(page.clone()),
    extract_resources(page.clone()),
    extract_timing_info(page.clone()),
    extract_security_info(page.clone()),
    async {
        let result: Result<String> = async {
            let title_value = page.evaluate("document.title").await
                .context("Failed to evaluate document.title")?
                .into_value()
                .map_err(|e| anyhow::anyhow!("Failed to get page title: {}", e))?;

            if let serde_json::Value::String(title) = title_value {
                Ok(title)
            } else {
                Ok(String::new())
            }
        }.await;
        result
    },
    extract_interactive_elements(page.clone()),
    extract_links(page.clone()),
)?;

// No need for intermediate Result handling - tokio::try_join! handles it!
```

**Pattern for link rewriting (lines 290-303):**
```rust
// BEFORE:
let (link_tx, link_rx) = tokio::sync::oneshot::channel();
let _link_task = config.link_rewriter.mark_links_for_discovery(&content, &url, move |result| {
    crate::on_result!(result, link_tx, "Failed to mark links for discovery");
});
let content_with_data_attrs = link_rx.await
    .map_err(|_| anyhow::anyhow!("Failed to mark links for discovery"))?;

// AFTER:
let content_with_data_attrs = config.link_rewriter
    .mark_links_for_discovery(&content, &url)
    .await?;
```

**Apply this conversion pattern throughout page_data.rs** - every callback should become a direct await.

### 9. SEARCH AND REPLACE

After converting all function definitions, search the entire codebase for remaining callback usage:

```bash
cd /Volumes/samsung_t9/kodegen/packages/citescrape
grep -r "on_result!" src/
grep -r "on_unit_result!" src/
```

Convert each occurrence from callback pattern to direct await.

## Verification Steps

After all changes, verify:

```bash
# 1. Check for any remaining callback patterns
cd /Volumes/samsung_t9/kodegen/packages/citescrape
grep -r "on_result!" src/          # Should return nothing
grep -r "on_unit_result!" src/     # Should return nothing
grep -r "executor::" src/          # Should return nothing

# 2. Verify compilation
cargo check --package kodegen_citescrape

# 3. Verify no broken imports
grep -r "use.*on_result" src/      # Should return nothing
grep -r "use.*executor" src/       # Should return nothing (except chromiumoxide executor)
```

## Definition of Done

- [ ] `runtime/mod.rs`: `on_result!` and `on_unit_result!` macros deleted
- [ ] `runtime/mod.rs`: `executor` module deleted  
- [ ] `runtime/async_task.rs`: `executor::register_waker` call removed
- [ ] `page_extractor/extractors.rs`: All 7 functions converted to pure async
- [ ] `page_extractor/link_rewriter.rs`: All 5 callback functions converted to pure async
- [ ] `crawl_engine/page_enhancer.rs`: `enhance_page` converted to pure async
- [ ] `crawl_engine/link_processor.rs`: `process_links_for_crawl` converted to pure async
- [ ] `search/engine.rs`: All 4 callback functions converted to pure async
- [ ] `page_extractor/page_data.rs`: All callback call sites converted to direct await
- [ ] All other call sites in the codebase updated
- [ ] No grep matches for `on_result!` in src/
- [ ] No grep matches for `on_unit_result!` in src/
- [ ] `cargo check --package kodegen_citescrape` passes without errors

## Benefits After Conversion

1. **Simpler Code**: No channel ceremony, no callback wrappers
2. **Better Error Handling**: Errors propagate via `?` operator instead of being logged and dropped
3. **Easier Composition**: Works naturally with `tokio::join!`, `tokio::select!`, `tokio::try_join!`
4. **Less Indirection**: Direct async/await matches chromiumoxide's native API
5. **Fewer Allocations**: No oneshot channels for every function call
6. **Better IDE Support**: Rust-analyzer understands async fn better than callback patterns

## References

- **chromiumoxide docs**: All Page methods are already `async fn` - see [tmp/chromiumoxide/src/page.rs](../tmp/chromiumoxide/src/page.rs)
- **Tokio async book**: https://tokio.rs/tokio/tutorial/async
- **Example conversions**: See thought process above for detailed before/after patterns
