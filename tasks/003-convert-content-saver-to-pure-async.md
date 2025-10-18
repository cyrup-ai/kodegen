# Task 003: Fix Broken Tests in compression.rs

## Status
INCOMPLETE - Tests Not Updated

## QA Rating: 7/10

### Issues Found

❌ **CRITICAL: Test cases still use old callback API and do not compile**

**Location:** `/Volumes/samsung_t9/kodegen/packages/citescrape/src/content_saver/compression.rs`

**Lines:** 246-253, 266-271

**Problem:**
Two test functions (`test_memory_efficiency_no_double_clone` and `test_no_clone_in_signature`) call `save_compressed_file` with the OLD callback-based API (4 parameters including a closure), but the function was converted to async and now takes only 3 parameters and returns `Result<CacheMetadata>`.

**Current Broken Code:**
```rust
#[test]
fn test_memory_efficiency_no_double_clone() {
    let test_data = vec![0u8; 1000];
    let temp_path = std::path::PathBuf::from("/tmp/test_compression");
    
    let _guard = save_compressed_file(
        test_data,
        &temp_path,
        "application/octet-stream",
        |_result| {  // ❌ This callback parameter doesn't exist anymore!
            // Callback for when compression completes
        }
    );
}

#[test]
fn test_no_clone_in_signature() {
    let data = vec![1, 2, 3];
    let temp_path = std::path::PathBuf::from("/tmp/test");
    let _guard = save_compressed_file(
        data,
        &temp_path,
        "application/octet-stream",
        |_| {}  // ❌ This callback parameter doesn't exist anymore!
    );
}
```

**Compilation Error:**
```
error[E0061]: this function takes 3 arguments but 4 arguments were supplied
   --> packages/citescrape/src/content_saver/compression.rs:246:22
```

## Required Fix

Update both test functions to use the new async API:

```rust
#[tokio::test]
async fn test_memory_efficiency_no_double_clone() {
    let test_data = vec![0u8; 1000];
    let temp_path = std::path::PathBuf::from("/tmp/test_compression");
    
    // Call async function with .await
    let result = save_compressed_file(
        test_data,
        &temp_path,
        "application/octet-stream",
    ).await;
    
    // Verify it works (ownership transferred, no clone needed)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_no_clone_in_signature() {
    let data = vec![1, 2, 3];
    let temp_path = std::path::PathBuf::from("/tmp/test");
    
    // Call async function with .await
    let result = save_compressed_file(
        data,
        &temp_path,
        "application/octet-stream",
    ).await;
    
    // Verify it works
    assert!(result.is_ok());
}
```

**Key Changes Required:**
1. Change `#[test]` to `#[tokio::test]`
2. Change `fn` to `async fn`
3. Remove the callback parameter (4th argument)
4. Add `.await` to the function call
5. Store result and add assertion to verify behavior

## Verification

After fixing, run:
```bash
cargo test --package kodegen_citescrape --lib content_saver::compression
```

Should see:
```
test content_saver::compression::benchmarks::test_memory_efficiency_no_double_clone ... ok
test content_saver::compression::benchmarks::test_no_clone_in_signature ... ok
```

## What Was Successfully Completed ✅

All main conversion work was completed successfully:

- ✅ `save_compressed_file` converted to pure async
- ✅ `save_json_data` converted to pure async  
- ✅ `save_page_data` converted to pure async
- ✅ `save_markdown_content` converted to pure async
- ✅ `save_html_content` converted to pure async
- ✅ `save_html_content_with_resources` converted to pure async
- ✅ Helper functions (`await_with_timeout`, `log_send_error`) removed
- ✅ Dependencies (`get_mirror_path`, `inline_all_resources`, etc.) converted
- ✅ No callbacks in production code
- ✅ No unwrap/expect in implementations
- ✅ Proper error handling with `?` operator
- ✅ All production code compiles

**Only the test cases were not updated.**
