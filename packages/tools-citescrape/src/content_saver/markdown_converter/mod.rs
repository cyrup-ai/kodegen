//! HTML to Markdown conversion pipeline - The ONE canonical implementation
//!
//! This module provides the complete pipeline for converting HTML to clean, well-formatted markdown:
//! 1. Extract main content using intelligent CSS selectors
//! 2. Clean HTML (remove scripts, styles, ads, tracking, etc.)
//! 3. Convert to markdown using html2md with custom post-processing
//! 4. Normalize markdown headings and handle edge cases
//!
//! # Usage
//!
//! ## Synchronous (for blocking contexts)
//! ```rust
//! use citescrape::content_saver::markdown_converter::{convert_html_to_markdown_sync, ConversionOptions};
//!
//! let html = "<html><body><h1>Title</h1><p>Content</p></body></html>";
//! let options = ConversionOptions::default();
//! let markdown = convert_html_to_markdown_sync(html, &options)?;
//! ```
//!
//! ## Asynchronous (recommended)
//! ```rust
//! use citescrape::content_saver::markdown_converter::{convert_html_to_markdown, ConversionOptions};
//!
//! let html = "<html><body><h1>Title</h1><p>Content</p></body></html>";
//! let options = ConversionOptions::default();
//! let markdown = convert_html_to_markdown(html, &options).await?;
//! ```
//!
//! ## Custom Configuration
//! ```rust
//! let options = ConversionOptions {
//!     extract_main_content: true,
//!     clean_html: true,
//!     preserve_tables: true,
//!     preserve_links: true,
//!     preserve_images: false,  // Remove images
//!     code_highlighting: true,
//!     process_headings: true,
//! };
//! let markdown = convert_html_to_markdown_sync(html, &options)?;
//! ```

use anyhow::Result;

// Declare sub-modules
mod html_preprocessing;
mod html_to_markdown;
mod markdown_postprocessing;

// Re-export sub-modules for advanced usage
pub use html_preprocessing::{clean_html_content, extract_main_content};
pub use html_to_markdown::MarkdownConverter;
pub use markdown_postprocessing::{
    extract_heading_level, normalize_heading_level, process_markdown_headings,
};

/// Configuration options for HTML to Markdown conversion
///
/// Controls all aspects of the conversion pipeline including content extraction,
/// HTML cleaning, markdown formatting, and post-processing.
#[derive(Debug, Clone)]
pub struct ConversionOptions {
    /// Extract main content using intelligent CSS selectors (default: true)
    ///
    /// When enabled, attempts to extract the primary content from the page,
    /// removing navigation, sidebars, headers, footers, etc.
    pub extract_main_content: bool,

    /// Clean HTML before conversion (default: true)
    ///
    /// Removes scripts, styles, ads, tracking pixels, social widgets,
    /// cookie notices, and other non-content elements.
    pub clean_html: bool,

    /// Preserve table formatting in markdown (default: true)
    pub preserve_tables: bool,

    /// Preserve hyperlinks in markdown (default: true)
    pub preserve_links: bool,

    /// Preserve images in markdown (default: true)
    pub preserve_images: bool,

    /// Enable code block syntax highlighting hints (default: true)
    pub code_highlighting: bool,

    /// Process and normalize markdown headings (default: true)
    ///
    /// Converts setext-style headings to ATX style, removes closing hashes,
    /// normalizes heading levels, and handles code fences properly.
    pub process_headings: bool,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            extract_main_content: true,
            clean_html: true,
            preserve_tables: true,
            preserve_links: true,
            preserve_images: true,
            code_highlighting: true,
            process_headings: true,
        }
    }
}

impl ConversionOptions {
    /// Create a new `ConversionOptions` with all features enabled
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable all optional processing (minimal conversion)
    ///
    /// Only performs basic HTML→Markdown conversion without extraction,
    /// cleaning, or post-processing. Useful for already-clean HTML.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            extract_main_content: false,
            clean_html: false,
            preserve_tables: true,
            preserve_links: true,
            preserve_images: true,
            code_highlighting: false,
            process_headings: false,
        }
    }

    /// Text-only mode: strips images and links
    #[must_use]
    pub fn text_only() -> Self {
        Self {
            extract_main_content: true,
            clean_html: true,
            preserve_tables: true,
            preserve_links: false,
            preserve_images: false,
            code_highlighting: true,
            process_headings: true,
        }
    }
}

/// Convert HTML to Markdown synchronously (blocking)
///
/// This is the ONE canonical function for HTML→Markdown conversion.
/// It orchestrates the complete 4-stage pipeline with built-in fallback handling.
///
/// # Pipeline Stages
///
/// 1. **Extract Main Content** (optional, controlled by `options.extract_main_content`)
///    - Uses intelligent CSS selectors to find primary content
///    - Falls back to full HTML if extraction fails
///
/// 2. **Clean HTML** (optional, controlled by `options.clean_html`)
///    - Removes scripts, styles, ads, tracking, social widgets
///    - Cleans up problematic HTML structures
///
/// 3. **Convert to Markdown** (always performed)
///    - Uses html2md library as base converter
///    - Applies custom post-processing (tables, lists, headings, code blocks)
///    - Built-in fallback to `html2md::parse_html` if post-processing fails
///
/// 4. **Process Headings** (optional, controlled by `options.process_headings`)
///    - Normalizes heading levels
///    - Converts setext→ATX style
///    - Removes closing hashes
///    - Handles code fences properly (including bug fixes `HTML_CLEANER_15`, `HTML_CLEANER_17`)
///
/// # Arguments
///
/// * `html` - Raw HTML content to convert
/// * `options` - Configuration controlling the conversion pipeline
///
/// # Returns
///
/// * `Ok(String)` - Clean, well-formatted markdown
/// * `Err(anyhow::Error)` - Only if ALL stages fail catastrophically (extremely rare)
///
/// # Examples
///
/// ```rust
/// let html = r#"
///     <html>
///         <body>
///             <article>
///                 <h1>My Article</h1>
///                 <p>This is <strong>important</strong> content.</p>
///             </article>
///         </body>
///     </html>
/// "#;
///
/// let markdown = convert_html_to_markdown_sync(html, &ConversionOptions::default())?;
/// assert!(markdown.contains("# My Article"));
/// assert!(markdown.contains("**important**"));
/// ```
pub fn convert_html_to_markdown_sync(html: &str, options: &ConversionOptions) -> Result<String> {
    // Stage 1: Extract main content (with fallback to full HTML)
    let main_html = if options.extract_main_content {
        match extract_main_content(html) {
            Ok(content) => content,
            Err(e) => {
                tracing::debug!("Main content extraction failed: {}, using full HTML", e);
                html.to_string()
            }
        }
    } else {
        html.to_string()
    };

    // Stage 2: Clean HTML (with passthrough if disabled)
    let clean_html = if options.clean_html {
        match clean_html_content(&main_html) {
            Ok(cleaned) => cleaned,
            Err(e) => {
                tracing::warn!("HTML cleaning failed: {}, using uncleaned HTML", e);
                main_html
            }
        }
    } else {
        main_html
    };

    // Stage 3: Convert to Markdown
    // The MarkdownConverter has built-in fallback to html2md::parse_html,
    // so we don't need to handle that here
    let converter = MarkdownConverter::new()
        .with_preserve_tables(options.preserve_tables)
        .with_preserve_links(options.preserve_links)
        .with_preserve_images(options.preserve_images)
        .with_code_highlighting(options.code_highlighting);

    let markdown = converter.convert_sync(&clean_html)?;

    // Stage 4: Process markdown headings (with passthrough if disabled)
    let final_markdown = if options.process_headings {
        process_markdown_headings(&markdown)
    } else {
        markdown
    };

    Ok(final_markdown)
}

/// Convert HTML to Markdown asynchronously
///
/// This is a thin async wrapper around the synchronous conversion logic.
/// Since HTML parsing/conversion is CPU-bound and typically fast (<100ms),
/// we call the sync function directly rather than using `spawn_blocking`.
///
/// # Arguments
///
/// * `html` - Raw HTML content to convert
/// * `options` - Configuration controlling the conversion pipeline
///
/// # Returns
///
/// * `Ok(String)` - Clean, well-formatted markdown
/// * `Err(anyhow::Error)` - Only if conversion fails catastrophically
///
/// # Examples
///
/// ```rust
/// let html = "<html><body><h1>Title</h1></body></html>";
/// let options = ConversionOptions::default();
/// let markdown = convert_html_to_markdown(html, &options).await?;
/// ```
pub async fn convert_html_to_markdown(html: &str, options: &ConversionOptions) -> Result<String> {
    // Direct call to sync version (it's fast enough)
    convert_html_to_markdown_sync(html, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_html_to_markdown_basic() {
        let html = r"
            <html>
                <body>
                    <h1>Test Title</h1>
                    <p>This is a <strong>test</strong> paragraph.</p>
                </body>
            </html>
        ";

        let result = convert_html_to_markdown_sync(html, &ConversionOptions::default());
        assert!(result.is_ok());

        let markdown = result.expect("Test operation should succeed");
        assert!(markdown.contains("# Test Title"));
        assert!(markdown.contains("**test**"));
    }

    #[test]
    fn test_convert_with_minimal_options() {
        let html = "<html><body><h2>Heading</h2><p>Content</p></body></html>";

        let result = convert_html_to_markdown_sync(html, &ConversionOptions::minimal());
        assert!(result.is_ok());

        let markdown = result.expect("Test operation should succeed");
        // With minimal options, headings might not be normalized
        assert!(markdown.contains("Heading"));
        assert!(markdown.contains("Content"));
    }

    #[test]
    fn test_convert_text_only_mode() {
        let html = r#"
            <html>
                <body>
                    <h1>Article</h1>
                    <p>Read more at <a href="https://example.com">example.com</a></p>
                    <img src="photo.jpg" alt="Photo">
                </body>
            </html>
        "#;

        let result = convert_html_to_markdown_sync(html, &ConversionOptions::text_only());
        assert!(result.is_ok());

        let markdown = result.expect("Test operation should succeed");
        // Text-only mode should strip images and convert links to plain text
        assert!(markdown.contains("Article"));
        assert!(!markdown.contains("![Photo]")); // No images
        // Links might be converted to plain text "example.com"
    }

    #[test]
    fn test_conversion_pipeline_stages() {
        let html = r"
            <html>
                <head><script>alert('test');</script></head>
                <body>
                    <nav>Navigation</nav>
                    <article>
                        <h1>Main Content</h1>
                        <p>Article text</p>
                    </article>
                    <footer>Footer</footer>
                </body>
            </html>
        ";

        let result = convert_html_to_markdown_sync(html, &ConversionOptions::default());
        assert!(result.is_ok());

        let markdown = result.expect("Test operation should succeed");
        // Should extract article content and remove nav/footer
        assert!(markdown.contains("# Main Content"));
        assert!(markdown.contains("Article text"));
        // Should NOT contain script tags
        assert!(!markdown.contains("alert"));
    }

    #[test]
    fn test_setext_heading_conversion() {
        let html = r"
            <html>
                <body>
                    <h1>Title</h1>
                </body>
            </html>
        ";

        // First convert to markdown (which might produce setext-style from html2md)
        let result = convert_html_to_markdown_sync(html, &ConversionOptions::default());
        assert!(result.is_ok());

        let markdown = result.expect("Test operation should succeed");
        // Should contain ATX-style heading (with process_headings enabled)
        // The actual format depends on html2md output, but our processor should normalize it
        assert!(markdown.contains("Title"));
    }

    #[test]
    fn test_code_fence_preservation() {
        let html = r#"
            <html>
                <body>
                    <h1>Code Example</h1>
                    <pre><code>
                    fn main() {
                        println!("Hello");
                    }
                    </code></pre>
                </body>
            </html>
        "#;

        let result = convert_html_to_markdown_sync(html, &ConversionOptions::default());
        assert!(result.is_ok());

        let markdown = result.expect("Test operation should succeed");
        assert!(markdown.contains("Code Example"));
        // Should contain code block markers
        assert!(markdown.contains("```") || markdown.contains("    ")); // Fenced or indented code
    }

    #[test]
    fn test_empty_html() {
        let html = "";
        let result = convert_html_to_markdown_sync(html, &ConversionOptions::default());
        assert!(result.is_ok());
        // Should return empty or minimal markdown, not error
    }

    #[test]
    fn test_malformed_html_resilience() {
        let html = "<html><body><h1>Unclosed heading<p>Paragraph</body>";
        let result = convert_html_to_markdown_sync(html, &ConversionOptions::default());
        assert!(result.is_ok());
        // Should handle gracefully, possibly with warnings
    }

    #[test]
    fn test_custom_options() {
        let options = ConversionOptions {
            extract_main_content: false,
            clean_html: false,
            preserve_tables: false,
            preserve_links: false,
            preserve_images: false,
            code_highlighting: false,
            process_headings: false,
        };

        let html = "<html><body><h1>Test</h1><a href='#'>Link</a></body></html>";
        let result = convert_html_to_markdown_sync(html, &options);
        assert!(result.is_ok());
    }
}
