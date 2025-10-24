use flate2::{Compression, GzBuilder};
use kodegen_tools_citescrape::content_saver::save_compressed_file;
use std::io::{Cursor, Write};
use std::time::Instant;

/// Generate typical HTML content of specified size
fn generate_html_content(size_kb: usize) -> Vec<u8> {
    let base_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Sample Web Page</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .content { padding: 15px; border: 1px solid #ddd; }
        h1 { color: #333; }
        p { line-height: 1.6; color: #666; }
    </style>
</head>
<body>
    <div class="content">
        <h1>Sample Content</h1>
        <p>This is typical web page content with various HTML elements, CSS styles, and text.</p>
    </div>
</body>
</html>"#;

    let mut content = Vec::new();
    let target_size = size_kb * 1024;

    while content.len() < target_size {
        content.extend_from_slice(base_html.as_bytes());
    }

    content.truncate(target_size);
    content
}

/// Compress data with specified level and return (duration, `compressed_size`)
fn compress_with_level(data: &[u8], level: u32) -> anyhow::Result<(std::time::Duration, usize)> {
    let start = Instant::now();

    let mut output = Cursor::new(Vec::new());
    let mut gz = GzBuilder::new()
        .filename("test.html")
        .write(&mut output, Compression::new(level));
    gz.write_all(data)?;
    gz.finish()?;

    let duration = start.elapsed();
    let compressed_size = output.into_inner().len();

    Ok((duration, compressed_size))
}

#[test]
fn benchmark_compression_levels_10kb() {
    let content = generate_html_content(10);
    println!("\n=== Compression Benchmark: 10KB HTML ===");
    println!("Original size: {} bytes", content.len());

    for level in [1, 3, 6, 9] {
        match compress_with_level(&content, level) {
            Ok((duration, size)) => {
                let ratio = (size as f64 / content.len() as f64) * 100.0;
                println!("Level {level}: {duration:?} | {size} bytes | {ratio:.1}% of original");
            }
            Err(e) => println!("Level {level}: Error - {e}"),
        }
    }
}

#[test]
fn benchmark_compression_levels_100kb() {
    let content = generate_html_content(100);
    println!("\n=== Compression Benchmark: 100KB HTML ===");
    println!("Original size: {} bytes", content.len());

    for level in [1, 3, 6, 9] {
        match compress_with_level(&content, level) {
            Ok((duration, size)) => {
                let ratio = (size as f64 / content.len() as f64) * 100.0;
                println!("Level {level}: {duration:?} | {size} bytes | {ratio:.1}% of original");
            }
            Err(e) => println!("Level {level}: Error - {e}"),
        }
    }
}

#[test]
fn benchmark_compression_levels_1mb() {
    let content = generate_html_content(1024);
    println!("\n=== Compression Benchmark: 1MB HTML ===");
    println!("Original size: {} bytes", content.len());

    for level in [1, 3, 6, 9] {
        match compress_with_level(&content, level) {
            Ok((duration, size)) => {
                let ratio = (size as f64 / content.len() as f64) * 100.0;
                println!("Level {level}: {duration:?} | {size} bytes | {ratio:.1}% of original");
            }
            Err(e) => println!("Level {level}: Error - {e}"),
        }
    }
}

#[test]
fn benchmark_compression_speedup() {
    let content = generate_html_content(500);
    println!("\n=== Compression Speed Comparison: 500KB HTML ===");

    let level_3 = compress_with_level(&content, 3).ok();
    let level_9 = compress_with_level(&content, 9).ok();

    if let (Some((time_3, size_3)), Some((time_9, size_9))) = (level_3, level_9) {
        let speedup = time_9.as_secs_f64() / time_3.as_secs_f64();
        let size_diff = ((size_3 as f64 - size_9 as f64) / size_9 as f64) * 100.0;

        println!("Level 3: {time_3:?} | {size_3} bytes");
        println!("Level 9: {time_9:?} | {size_9} bytes");
        println!("Speedup: {speedup:.2}x faster");
        println!("Size difference: Level 3 is {size_diff:.1}% larger");
    }
}

#[tokio::test]
async fn test_memory_efficiency_no_double_clone() {
    // This test verifies that save_compressed_file accepts owned Vec<u8>
    // without requiring clones by the caller, eliminating the double-clone issue.

    // Create test data - ownership will be transferred
    let test_data = vec![0u8; 1000];
    let temp_path = std::path::PathBuf::from("/tmp/test_compression");

    // Call async function with .await
    let result = save_compressed_file(
        test_data, // Ownership transferred - no clone by caller needed!
        &temp_path,
        "application/octet-stream",
        true, // Enable compression to test the compression code path
    )
    .await;

    // Verify it works (ownership transferred, no clone needed)
    assert!(result.is_ok());

    // The fact that this compiles proves ownership transfer works.
    // Attempting to use test_data here would cause a compile error.
}

#[tokio::test]
async fn test_no_clone_in_signature() {
    // Compile-time test: if this compiles, owned data works correctly
    let data = vec![1, 2, 3];
    let temp_path = std::path::PathBuf::from("/tmp/test");

    // Call async function with .await
    let result = save_compressed_file(
        data, // Move ownership - no clone required
        &temp_path,
        "application/octet-stream",
        true, // Enable compression to test the compression code path
    )
    .await;

    // Verify it works
    assert!(result.is_ok());

    // data is moved, this should NOT compile if you uncomment:
    // println!("{:?}", data);  // ❌ Would fail: value was moved
}
