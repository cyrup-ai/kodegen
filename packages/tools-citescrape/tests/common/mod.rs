//! Test utilities and helper functions for the citescrape test suite

use anyhow::Result;
use chromiumoxide::{Browser, BrowserConfig};
use mockito::{Mock, Server};
use std::path::Path;
use tempfile::TempDir;

/// Creates a temporary directory for test output
#[allow(dead_code)]
pub fn create_test_dir() -> Result<TempDir> {
    Ok(TempDir::new()?)
}

/// Creates a test HTML document with specified content
#[allow(dead_code)]
pub fn create_test_html(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
</head>
<body>
    {}
</body>
</html>"#,
        html_escape::encode_text(title),
        body
    )
}

/// Creates a complex HTML document for testing various elements
#[allow(dead_code)]
pub fn create_complex_html() -> String {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>Complex Test Page</title>
</head>
<body>
    <h1>Main Heading</h1>
    <p>This is a paragraph with <strong>bold</strong> and <em>italic</em> text.</p>
    
    <h2>Subheading</h2>
    <ul>
        <li>List item 1</li>
        <li>List item 2</li>
        <li>List item 3</li>
    </ul>
    
    <h3>Code Example</h3>
    <pre><code class="language-rust">
fn main() {
    println!("Hello, world!");
}
    </code></pre>
    
    <h2>Links and Images</h2>
    <p>Visit <a href="https://example.com">our website</a> for more info.</p>
    <img src="/images/test.png" alt="Test image" />
    
    <h2>Table</h2>
    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>Value</th>
            </tr>
        </thead>
        <tbody>
            <tr>
                <td>Item 1</td>
                <td>100</td>
            </tr>
            <tr>
                <td>Item 2</td>
                <td>200</td>
            </tr>
        </tbody>
    </table>
</body>
</html>"#
        .to_string()
}

/// Sets up a mock HTTP server with predefined responses
#[allow(dead_code)]
pub async fn setup_mock_server() -> Result<mockito::ServerGuard> {
    let server = Server::new_async().await;
    Ok(server)
}

/// Creates a mock endpoint that returns HTML content
#[allow(dead_code)]
pub fn create_html_mock(server: &mut Server, path: &str, html: &str) -> Mock {
    server
        .mock("GET", path)
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body(html)
        .create()
}

/// Creates a mock endpoint that returns a redirect
#[allow(dead_code)]
pub fn create_redirect_mock(server: &mut Server, from: &str, to: &str) -> Mock {
    server
        .mock("GET", from)
        .with_status(301)
        .with_header("location", to)
        .create()
}

/// Creates a mock endpoint that returns an error
#[allow(dead_code)]
pub fn create_error_mock(server: &mut Server, path: &str, status: usize) -> Mock {
    server
        .mock("GET", path)
        .with_status(status)
        .with_body("Error")
        .create()
}

/// Launches a headless Chrome browser for testing
#[allow(dead_code)]
pub async fn launch_test_browser() -> Result<(Browser, chromiumoxide::Handler)> {
    let (browser, handler) = Browser::launch(
        BrowserConfig::builder()
            .no_sandbox()
            .build()
            .map_err(|e| anyhow::anyhow!(e))?,
    )
    .await?;

    // Note: Handler is returned to caller to manage
    Ok((browser, handler))
}

/// Creates test configuration for crawling
#[allow(dead_code)]
pub async fn create_test_config(
    storage_dir: &Path,
    start_url: &str,
) -> kodegen_tools_citescrape::config::CrawlConfig {
    kodegen_tools_citescrape::config::CrawlConfig::builder()
        .storage_dir(storage_dir.to_path_buf())
        .start_url(start_url)
        .limit(Some(10))
        .build()
        .expect("Failed to create test config")
}

/// Compares two markdown strings, normalizing whitespace
#[allow(dead_code)]
pub fn assert_markdown_eq(actual: &str, expected: &str) {
    let normalize = |s: &str| {
        s.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    };

    let actual_normalized = normalize(actual);
    let expected_normalized = normalize(expected);

    if actual_normalized != expected_normalized {
        println!("=== ACTUAL ===\n{actual}\n");
        println!("=== EXPECTED ===\n{expected}\n");
        panic!("Markdown content does not match");
    }
}

/// Verifies that a file exists and has content
#[allow(dead_code)]
pub async fn assert_file_exists_with_content(path: &Path) -> Result<String> {
    assert!(path.exists(), "File does not exist: {path:?}");
    let content = tokio::fs::read_to_string(path).await?;
    assert!(!content.is_empty(), "File is empty: {path:?}");
    Ok(content)
}

/// Creates a sample robots.txt for testing
#[allow(dead_code)]
pub fn create_robots_txt(disallow_paths: &[&str]) -> String {
    let mut content = String::from("User-agent: *\n");
    for path in disallow_paths {
        content.push_str(&format!("Disallow: {path}\n"));
    }
    content
}

/// Helper to create test URLs
#[allow(dead_code)]
pub fn test_url(server: &Server, path: &str) -> String {
    format!("{}{}", server.url(), path)
}

/// Waits for a condition to be true with timeout
#[allow(dead_code)]
pub async fn wait_for_condition<F, Fut>(mut check: F, timeout_secs: u64) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if check().await {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Err(anyhow::anyhow!("Timeout waiting for condition"))
}

/// Creates a test file tree for mirror testing
#[allow(dead_code)]
pub async fn create_test_file_tree(root: &Path) -> Result<()> {
    let files = vec![
        "index.html",
        "about.html",
        "blog/post1.html",
        "blog/post2.html",
        "assets/style.css",
        "assets/script.js",
        "images/logo.png",
    ];

    for file_path in files {
        let full_path = root.join(file_path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&full_path, format!("Test content for {file_path}")).await?;
    }

    Ok(())
}

/// Counts files recursively in a directory
#[allow(dead_code)]
pub fn count_files_recursive(dir: &Path) -> futures::future::BoxFuture<'_, Result<usize>> {
    Box::pin(async move {
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                count += 1;
            } else if path.is_dir() {
                count += count_files_recursive(&path).await?;
            }
        }

        Ok(count)
    })
}
