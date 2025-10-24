//! HTML to Markdown conversion functionality.
//!
//! This module wraps the html2md library and adds additional post-processing
//! to produce clean, well-formatted markdown output.

use anyhow::Result;
use html2md::parse_html;
use regex::Regex;
use std::sync::LazyLock;

// Compile regex patterns once at first use
// These are syntactically valid hardcoded patterns - if they fail, it's a compile-time bug
static EMPTY_LINES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\n{3,}")
        .expect("BUG: hardcoded regex r\"\\n{3,}\" is invalid - this is a compile-time bug")
});

static SPACE_AFTER_LIST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(\s*[-*+])\s*").expect(
        "BUG: hardcoded regex r\"(?m)^(\\s*[-*+])\\s*\" is invalid - this is a compile-time bug",
    )
});

static HEADING_SPACE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(#+)([^ ])")
        .expect("BUG: hardcoded regex r\"(?m)^(#+)([^ ])\" is invalid - this is a compile-time bug")
});

static TABLE_ALIGN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\|(\s*:?-+:?\s*\|)+").expect(
        "BUG: hardcoded regex r\"\\|(\\s*:?-+:?\\s*\\|)+\" is invalid - this is a compile-time bug",
    )
});

static CODE_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"```([a-zA-Z]*)\n").expect(
        "BUG: hardcoded regex r\"```([a-zA-Z]*)\\n\" is invalid - this is a compile-time bug",
    )
});

static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[([^\]]+)\]\([^\)]+\)")
        .expect("BUG: hardcoded regex r\"\\[([^\\]]+)\\]\\([^\\)]+\\)\" is invalid - this is a compile-time bug")
});

static IMAGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"!\[[^\]]*\]\([^\)]+\)")
        .expect("BUG: hardcoded regex r\"!\\[[^\\]]*\\]\\([^\\)]+\\)\" is invalid - this is a compile-time bug")
});

/// HTML to Markdown converter with configurable options
pub struct MarkdownConverter {
    preserve_tables: bool,
    preserve_links: bool,
    preserve_images: bool,
    code_highlighting: bool,
}

impl Default for MarkdownConverter {
    fn default() -> Self {
        Self {
            preserve_tables: true,
            preserve_links: true,
            preserve_images: true,
            code_highlighting: true,
        }
    }
}

impl MarkdownConverter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_preserve_tables(mut self, preserve: bool) -> Self {
        self.preserve_tables = preserve;
        self
    }

    #[must_use]
    pub fn with_preserve_links(mut self, preserve: bool) -> Self {
        self.preserve_links = preserve;
        self
    }

    #[must_use]
    pub fn with_preserve_images(mut self, preserve: bool) -> Self {
        self.preserve_images = preserve;
        self
    }

    #[must_use]
    pub fn with_code_highlighting(mut self, highlight: bool) -> Self {
        self.code_highlighting = highlight;
        self
    }

    /// Convert HTML to Markdown synchronously (with built-in fallback)
    pub fn convert_sync(&self, html: &str) -> Result<String> {
        // First pass: Convert HTML to basic markdown
        let mut markdown = parse_html(html);

        // Clean up the markdown
        markdown = Self::clean_markdown_static(&markdown);

        // Handle code blocks
        if self.code_highlighting {
            markdown = CODE_BLOCK.replace_all(&markdown, "```$1\n").to_string();
        }

        // Clean up lists
        markdown = SPACE_AFTER_LIST.replace_all(&markdown, "$1 ").to_string();

        // Fix heading spacing
        markdown = HEADING_SPACE.replace_all(&markdown, "$1 $2").to_string();

        // Handle tables if enabled
        if self.preserve_tables {
            markdown = Self::format_tables_static(&markdown);
        }

        // Remove excessive newlines
        markdown = EMPTY_LINES.replace_all(&markdown, "\n\n").to_string();

        // Handle links and images based on settings
        if !self.preserve_links {
            markdown = Self::remove_links_static(&markdown);
        }
        if !self.preserve_images {
            markdown = Self::remove_images_static(&markdown);
        }

        Ok(markdown.trim().to_string())
    }

    /// Convert HTML to Markdown asynchronously
    ///
    /// Performs the same conversion as `convert_sync()` but in an async context.
    /// Since the work is CPU-bound, this simply calls the sync version.
    ///
    /// # Arguments
    ///
    /// * `html` - Raw HTML content to convert
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Converted markdown
    /// * `Err(anyhow::Error)` - Conversion error
    pub async fn convert(&self, html: &str) -> Result<String> {
        self.convert_sync(html)
    }

    fn clean_markdown_static(markdown: &str) -> String {
        let mut cleaned = markdown.to_string();

        // Remove HTML comments
        cleaned = cleaned
            .lines()
            .filter(|line| !line.trim_start().starts_with("<!--"))
            .collect::<Vec<_>>()
            .join("\n");

        // Fix list indentation
        cleaned = cleaned
            .lines()
            .map(|line| {
                if line.trim_start().starts_with(['-', '*', '+']) {
                    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
                    format!("{}{}", " ".repeat(indent), line.trim_start())
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        cleaned
    }

    fn format_tables_static(markdown: &str) -> String {
        let mut formatted = markdown.to_string();

        // Ensure table headers are properly aligned
        formatted = TABLE_ALIGN
            .replace_all(&formatted, |caps: &regex::Captures| {
                caps[0]
                    .trim_matches('|')
                    .split('|')
                    .map(str::trim)
                    .map(|col| {
                        if col.starts_with(':') && col.ends_with(':') {
                            "|:---:|"
                        } else if col.starts_with(':') {
                            "|:---|"
                        } else if col.ends_with(':') {
                            "|---:|"
                        } else {
                            "|---|"
                        }
                    })
                    .collect::<String>()
            })
            .to_string();

        formatted
    }

    fn remove_links_static(markdown: &str) -> String {
        // Convert [text](url) to just text
        LINK_RE.replace_all(markdown, "$1").to_string()
    }

    fn remove_images_static(markdown: &str) -> String {
        // Remove ![alt](url) completely
        IMAGE_RE.replace_all(markdown, "").to_string()
    }
}
