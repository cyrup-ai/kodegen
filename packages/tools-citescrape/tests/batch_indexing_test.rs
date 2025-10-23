use anyhow::Result;
use futures::StreamExt;
use kodegen_tools_citescrape::config::CrawlConfig;
use kodegen_tools_citescrape::search::{
    engine::SearchEngine,
    indexer::{BatchConfig, MarkdownIndexer},
    types::IndexingPhase,
};
use std::fs;
use tempfile::TempDir;

/// Create a test `crawl_output` directory structure with sample files
fn create_test_crawl_output(dir: &TempDir) -> Result<()> {
    let crawl_output = dir.path().join("crawl_output");

    // Create various domain/path structures
    let test_files = vec![
        (
            "example.com",
            "",
            "# Example Homepage\n\nWelcome to example.com",
        ),
        (
            "example.com",
            "docs",
            "# Documentation\n\nThis is the docs section",
        ),
        (
            "example.com",
            "docs/api",
            "# API Reference\n\nAPI documentation here",
        ),
        ("blog.example.com", "", "# Blog\n\nLatest blog posts"),
        (
            "blog.example.com",
            "2024/01/post1",
            "# First Post\n\nContent of first post",
        ),
        ("test.com", "page", "# Test Page\n\nTest content"),
    ];

    for (domain, path, content) in test_files {
        let file_dir = if path.is_empty() {
            crawl_output.join(domain)
        } else {
            crawl_output.join(domain).join(path)
        };

        fs::create_dir_all(&file_dir)?;

        // Write compressed markdown file
        let file_path = file_dir.join("index.md.gz");
        let compressed = compress_string(content)?;
        fs::write(file_path, compressed)?;
    }

    // Add a corrupted file for error testing
    // Create data with gzip magic bytes (0x1f, 0x8b) but invalid content
    // This will trigger decompression which will fail
    let corrupted_dir = crawl_output.join("corrupted.com");
    fs::create_dir_all(&corrupted_dir)?;
    let invalid_gzip_data = vec![0x1f, 0x8b, 0xFF, 0xFF, 0xFF, 0xFF]; // Magic bytes + invalid data
    fs::write(corrupted_dir.join("index.md.gz"), invalid_gzip_data)?;

    Ok(())
}

/// Compress a string to gzip format
fn compress_string(content: &str) -> Result<Vec<u8>> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content.as_bytes())?;
    Ok(encoder.finish()?)
}

#[tokio::test]
async fn test_discover_markdown_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_crawl_output(&temp_dir)?;

    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .build()
        .map_err(anyhow::Error::msg)?;

    let engine = SearchEngine::create(&config).await?;
    let indexer = MarkdownIndexer::new(engine);

    let crawl_output = temp_dir.path().join("crawl_output");
    let discovered: Vec<_> = indexer
        .discover_markdown_files_stream(&crawl_output)
        .collect::<Result<Vec<_>, _>>()?;

    // Should find 6 valid files plus 1 corrupted
    assert_eq!(discovered.len(), 7);

    // Check URL extraction
    let urls: Vec<String> = discovered.iter().map(|(_, url)| url.to_string()).collect();
    assert!(urls.contains(&"https://example.com/".to_string()));
    assert!(urls.contains(&"https://example.com/docs/".to_string()));
    assert!(urls.contains(&"https://example.com/docs/api/".to_string()));
    assert!(urls.contains(&"https://blog.example.com/".to_string()));
    assert!(urls.contains(&"https://blog.example.com/2024/01/post1/".to_string()));
    assert!(urls.contains(&"https://test.com/page/".to_string()));
    assert!(urls.contains(&"https://corrupted.com/".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_batch_index_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_crawl_output(&temp_dir)?;

    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .build()
        .map_err(anyhow::Error::msg)?;

    let engine = SearchEngine::create(&config).await?;
    let indexer = MarkdownIndexer::new(engine.clone());

    let crawl_output = temp_dir.path().join("crawl_output");
    let batch_config = BatchConfig {
        batch_size: 3,
        ..Default::default()
    };
    let (mut stream, _cancel_handle) = indexer.batch_index_directory(crawl_output, batch_config);

    let mut discovered = false;
    let mut indexing_started = false;
    let mut completed = false;
    let mut total_processed = 0;
    let mut total_failed = 0;

    while let Some(result) = stream.next().await {
        match result {
            Ok(progress) => {
                eprintln!(
                    "Progress phase: {:?}, processed: {}, failed: {}",
                    progress.phase, progress.processed, progress.failed
                );
                match progress.phase {
                    IndexingPhase::Discovering => {
                        discovered = true;
                        assert!(progress.files_discovered > 0);
                    }
                    IndexingPhase::Indexing => {
                        indexing_started = true;
                        assert!(progress.total > 0);
                    }
                    IndexingPhase::Complete => {
                        completed = true;
                        total_processed = progress.processed;
                        total_failed = progress.failed;
                    }
                    _ => {}
                }
            }
            Err(e) => {
                panic!("Unexpected error during batch indexing: {e}");
            }
        }
    }

    assert!(discovered, "Discovery phase should have been reported");
    assert!(indexing_started, "Indexing phase should have been reported");
    assert!(completed, "Completion should have been reported");

    // We should have processed 6 valid files
    assert_eq!(total_processed, 6);
    // And failed on 1 corrupted file
    assert_eq!(total_failed, 1);

    // Verify index contains the documents
    // Give a small delay to ensure commit is fully processed
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let stats = indexer.get_index_stats().await?;
    eprintln!(
        "Stats: num_documents={}, num_segments={}",
        stats.num_documents, stats.num_segments
    );
    assert_eq!(stats.num_documents, 6);

    Ok(())
}

#[tokio::test]
async fn test_batch_index_empty_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir_all(&empty_dir)?;

    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .build()
        .map_err(anyhow::Error::msg)?;

    let engine = SearchEngine::create(&config).await?;
    let indexer = MarkdownIndexer::new(engine);

    let batch_config = BatchConfig {
        batch_size: 10,
        ..Default::default()
    };
    let (mut stream, _cancel_handle) = indexer.batch_index_directory(empty_dir, batch_config);

    let mut progress_count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(progress) => {
                progress_count += 1;
                if progress.phase == IndexingPhase::Complete {
                    assert_eq!(progress.processed, 0);
                    assert_eq!(progress.failed, 0);
                    assert_eq!(progress.files_discovered, 0);
                }
            }
            Err(e) => {
                panic!("Unexpected error: {e}");
            }
        }
    }

    assert!(progress_count > 0, "Should have received progress updates");
    Ok(())
}

#[tokio::test]
async fn test_batch_size_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    create_test_crawl_output(&temp_dir)?;

    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .build()
        .map_err(anyhow::Error::msg)?;

    let engine = SearchEngine::create(&config).await?;
    let indexer = MarkdownIndexer::new(engine);

    // Use a small batch size to test batching
    let crawl_output = temp_dir.path().join("crawl_output");
    let batch_config = BatchConfig {
        batch_size: 2,
        ..Default::default()
    };
    let (mut stream, _cancel_handle) = indexer.batch_index_directory(crawl_output, batch_config);

    let mut batch_commits = 0;
    while let Some(result) = stream.next().await {
        if let Ok(progress) = result {
            // Count how many times we're processing files
            if progress.phase == IndexingPhase::Indexing && progress.processed > 0 {
                // With batch size 2 and 7 files, we should see multiple updates
                batch_commits += 1;
            }
        }
    }

    // Should have multiple batches with batch size 2 and 7 files
    assert!(batch_commits >= 3, "Should have processed multiple batches");

    Ok(())
}

#[tokio::test]
async fn test_url_deduplication() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let crawl_output = temp_dir.path().join("crawl_output");

    // Create duplicate files with same URL
    let domain = "duplicate.com";
    let path1 = crawl_output.join(domain).join("path1");
    let path2 = crawl_output.join(domain).join("path2");

    fs::create_dir_all(&path1)?;
    fs::create_dir_all(&path2)?;

    // Both will map to https://duplicate.com/path1/ and https://duplicate.com/path2/
    let content = compress_string("# Duplicate Content")?;
    fs::write(path1.join("index.md.gz"), &content)?;
    fs::write(path2.join("index.md.gz"), &content)?;

    let config = CrawlConfig::builder()
        .storage_dir(temp_dir.path().to_path_buf())
        .start_url("https://example.com")
        .build()
        .map_err(anyhow::Error::msg)?;

    let engine = SearchEngine::create(&config).await?;
    let indexer = MarkdownIndexer::new(engine);

    let discovered: Vec<_> = indexer
        .discover_markdown_files_stream(&crawl_output)
        .collect::<Result<Vec<_>, _>>()?;

    // Should have 2 unique URLs
    assert_eq!(discovered.len(), 2);

    let urls: Vec<String> = discovered.iter().map(|(_, url)| url.to_string()).collect();
    assert!(urls.contains(&"https://duplicate.com/path1/".to_string()));
    assert!(urls.contains(&"https://duplicate.com/path2/".to_string()));

    Ok(())
}
