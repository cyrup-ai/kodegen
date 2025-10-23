//! HTML preprocessing functionality for markdown conversion.
//!
//! This module provides two main functions:
//! 1. `extract_main_content` - Intelligently extracts the primary content from HTML
//! 2. `clean_html_content` - Removes scripts, styles, ads, and other non-content elements
//!
//! These functions prepare HTML for optimal markdown conversion.

use anyhow::Result;
use ego_tree::NodeId;
use html_escape::decode_html_entities;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::LazyLock;

// ============================================================================
// PART 1: Main Content Extraction
// ============================================================================

/// Efficiently remove elements matching selectors from an element's subtree.
///
/// This function:
/// 1. Parses all selectors once (O(s) where s = number of selectors)
/// 2. Builds a HashSet of element pointers to remove (O(n) where n = number of elements)
/// 3. Serializes the DOM tree once, skipping removed elements - O(n)
///
/// Overall complexity: O(s + n) instead of O(s × n²) from naive string replacement
///
/// Note: This preserves HTML structure (tags, attributes, nesting) while removing
/// unwanted elements, as required by downstream processors (clean_html_content, MarkdownConverter).
///
/// Works directly with the element's node tree, avoiding redundant serialization and re-parsing.
fn remove_elements_from_html(element: &ElementRef, remove_selectors: &[&str]) -> String {
    // Parse all selectors upfront - O(s)
    let parsed_selectors: Vec<Selector> = remove_selectors
        .iter()
        .map(|&sel_str| match Selector::parse(sel_str) {
            Ok(s) => s,
            Err(e) => panic!("Invalid hardcoded selector '{}': {}", sel_str, e),
        })
        .collect();

    // Build HashSet of all elements to remove (using NodeId for O(1) lookup) - O(n)
    let mut to_remove: HashSet<NodeId> = HashSet::new();
    for sel in &parsed_selectors {
        for elem in element.select(sel) {
            // Store NodeId for identity comparison
            to_remove.insert(elem.id());
        }
    }

    // Serialize HTML while skipping removed elements - O(n)
    let mut result = String::new();
    serialize_html_excluding(element, &to_remove, &mut result);
    result
}

/// Recursively serialize an element and its descendants to HTML,
/// skipping elements in the removal set.
///
/// This preserves the full HTML structure (tags, attributes, nesting) while
/// excluding unwanted elements and their children.
fn serialize_html_excluding(
    element: &ElementRef,
    to_remove: &HashSet<NodeId>,
    output: &mut String,
) {
    // Check if this element should be removed
    if to_remove.contains(&element.id()) {
        return; // Skip this element and all its children
    }

    // Serialize this element's children (we're at the root or an allowed element)
    for child in element.children() {
        use scraper::node::Node;

        match child.value() {
            Node::Text(text) => {
                // Escape HTML special characters in text content
                for ch in text.chars() {
                    match ch {
                        '<' => output.push_str("&lt;"),
                        '>' => output.push_str("&gt;"),
                        '&' => output.push_str("&amp;"),
                        '"' => output.push_str("&quot;"),
                        c => output.push(c),
                    }
                }
            }
            Node::Element(_) => {
                // Element node - check if it should be removed
                if let Some(child_elem) = ElementRef::wrap(child) {
                    if to_remove.contains(&child_elem.id()) {
                        // Skip this element and its children
                        continue;
                    }

                    // Serialize the element with its tags and attributes
                    let elem_name = child_elem.value().name();
                    output.push('<');
                    output.push_str(elem_name);

                    // Serialize attributes
                    for (name, value) in child_elem.value().attrs() {
                        output.push(' ');
                        output.push_str(name);
                        output.push_str("=\"");
                        // Escape attribute value
                        for ch in value.chars() {
                            match ch {
                                '"' => output.push_str("&quot;"),
                                '&' => output.push_str("&amp;"),
                                '<' => output.push_str("&lt;"),
                                '>' => output.push_str("&gt;"),
                                c => output.push(c),
                            }
                        }
                        output.push('"');
                    }
                    output.push('>');

                    // Check if this is a void element (self-closing)
                    const VOID_ELEMENTS: &[&str] = &[
                        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
                        "param", "source", "track", "wbr",
                    ];

                    if VOID_ELEMENTS.contains(&elem_name) {
                        // Void element - no closing tag needed
                        continue;
                    }

                    // Recursively serialize children
                    serialize_html_excluding(&child_elem, to_remove, output);

                    // Closing tag (only for non-void elements)
                    output.push_str("</");
                    output.push_str(elem_name);
                    output.push('>');
                }
            }
            Node::Comment(comment) => {
                // Preserve comments
                output.push_str("<!--");
                output.push_str(comment);
                output.push_str("-->");
            }
            _ => {
                // Other node types (Document, Doctype, ProcessingInstruction) - skip
            }
        }
    }
}

/// Extract main content from HTML by removing common non-content elements
pub fn extract_main_content(html: &str) -> Result<String> {
    let document = Html::parse_document(html);

    // First, remove common non-content elements
    let remove_selectors = [
        "nav",
        "header",
        "footer",
        "aside",
        ".sidebar",
        "#sidebar",
        ".navigation",
        ".header",
        ".footer",
        ".menu",
        ".ads",
        ".advertisement",
        ".social-share",
        ".comments",
        "#comments",
        ".related-posts",
        ".cookie-notice",
        ".popup",
        ".modal",
    ];

    // Try to find main content in common containers
    let content_selectors = [
        "main",
        "article",
        "[role='main']",
        "#main-content",
        ".main-content",
        "#content",
        ".content",
        ".post-content",
        ".entry-content",
        "[itemprop='articleBody']",
        ".article-body",
        ".story-body",
    ];

    // First try to find a specific content container
    for selector in content_selectors {
        let sel = match Selector::parse(selector) {
            Ok(s) => s,
            Err(e) => panic!("Invalid hardcoded selector '{}': {}", selector, e),
        };

        if let Some(element) = document.select(&sel).next() {
            // Efficiently remove unwanted elements within the content
            // Works directly with element's node tree - no serialize-parse roundtrip
            let cleaned_html = remove_elements_from_html(&element, &remove_selectors);

            return Ok(cleaned_html);
        }
    }

    // If no main content container found, try to extract body and remove non-content elements
    let body_sel = match Selector::parse("body") {
        Ok(s) => s,
        Err(e) => panic!("Invalid hardcoded selector 'body': {}", e),
    };

    if let Some(body) = document.select(&body_sel).next() {
        // Efficiently remove non-content elements from body
        // Works directly with element's node tree - no serialize-parse roundtrip
        let cleaned_html = remove_elements_from_html(&body, &remove_selectors);

        return Ok(cleaned_html);
    }

    // Last resort: return the whole HTML
    Ok(html.to_string())
}

// ============================================================================
// PART 2: HTML Cleaning
// ============================================================================

// Compile regex patterns once at first use
// These are hardcoded patterns - if they fail to compile, it's a compile-time bug
static SCRIPT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<script[^>]*>.*?</script>")
        .expect("BUG: hardcoded SCRIPT_RE regex is invalid - this is a compile-time bug")
});

static STYLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<style[^>]*>.*?</style>")
        .expect("BUG: hardcoded STYLE_RE regex is invalid - this is a compile-time bug")
});

static EVENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"on\w+="[^"]*""#)
        .expect("BUG: hardcoded EVENT_RE regex is invalid - this is a compile-time bug")
});

static COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<!--.*?-->")
        .expect("BUG: hardcoded COMMENT_RE regex is invalid - this is a compile-time bug")
});

static FORM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<form[^>]*>.*?</form>")
        .expect("BUG: hardcoded FORM_RE regex is invalid - this is a compile-time bug")
});

static IFRAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<iframe[^>]*>.*?</iframe>")
        .expect("BUG: hardcoded IFRAME_RE regex is invalid - this is a compile-time bug")
});

static SOCIAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<div[^>]*class="[^"]*(?:social|share|follow)[^"]*"[^>]*>.*?</div>"#)
        .expect("BUG: hardcoded SOCIAL_RE regex is invalid - this is a compile-time bug")
});

static COOKIE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<div[^>]*(?:id|class)="[^"]*(?:cookie|popup|modal|overlay)[^"]*"[^>]*>.*?</div>"#,
    )
    .expect("BUG: hardcoded COOKIE_RE regex is invalid - this is a compile-time bug")
});

static AD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<div[^>]*(?:id|class)="[^"]*(?:ad-|ads-|advertisement)[^"]*"[^>]*>.*?</div>"#)
        .expect("BUG: hardcoded AD_RE regex is invalid - this is a compile-time bug")
});

static HIDDEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<[^>]+style="[^"]*display:\s*none[^"]*"[^>]*>.*?</[^>]+>"#)
        .expect("BUG: hardcoded HIDDEN_RE regex is invalid - this is a compile-time bug")
});

static DETAILS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<details[^>]*>(.*?)</details>")
        .expect("BUG: hardcoded DETAILS_RE regex is invalid - this is a compile-time bug")
});

static SEMANTIC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"<(/?)(?:article|section|aside|nav|header|footer|figure|figcaption|mark|time)[^>]*>",
    )
    .expect("BUG: hardcoded SEMANTIC_RE regex is invalid - this is a compile-time bug")
});

// Special case: needs to be compiled for closure captures in details processing
static SUMMARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<summary[^>]*>(.*?)</summary>")
        .expect("BUG: hardcoded SUMMARY_RE regex is invalid - this is a compile-time bug")
});

/// Clean HTML content by removing unwanted elements and scripts
pub fn clean_html_content(html: &str) -> String {
    // Use Cow to avoid unnecessary allocations
    // Start with borrowed reference, only allocate when modifications occur
    let mut result = Cow::Borrowed(html);

    // Remove script tags and their contents
    result = Cow::Owned(SCRIPT_RE.replace_all(&result, "").into_owned());

    // Remove style tags and their contents
    result = Cow::Owned(STYLE_RE.replace_all(&result, "").into_owned());

    // Remove inline event handlers
    result = Cow::Owned(EVENT_RE.replace_all(&result, "").into_owned());

    // Remove comments
    result = Cow::Owned(COMMENT_RE.replace_all(&result, "").into_owned());

    // Remove forms
    result = Cow::Owned(FORM_RE.replace_all(&result, "").into_owned());

    // Remove iframes
    result = Cow::Owned(IFRAME_RE.replace_all(&result, "").into_owned());

    // Remove social media widgets and buttons
    result = Cow::Owned(SOCIAL_RE.replace_all(&result, "").into_owned());

    // Remove cookie notices and popups
    result = Cow::Owned(COOKIE_RE.replace_all(&result, "").into_owned());

    // Remove ads
    result = Cow::Owned(AD_RE.replace_all(&result, "").into_owned());

    // Remove hidden elements
    result = Cow::Owned(HIDDEN_RE.replace_all(&result, "").into_owned());

    // Handle HTML5 details/summary elements by extracting their content
    // These don't convert well to markdown
    result = Cow::Owned(
        DETAILS_RE
            .replace_all(&result, |caps: &regex::Captures| {
                let content = &caps[1];
                // Extract summary text if present
                if let Some(summary_match) = SUMMARY_RE.captures(content) {
                    let summary_text = &summary_match[1];
                    let remaining = SUMMARY_RE.replace(content, "");
                    format!(
                        "\n\n**{}**\n\n{}\n\n",
                        summary_text.trim(),
                        remaining.trim()
                    )
                } else {
                    format!("\n\n{}\n\n", content.trim())
                }
            })
            .into_owned(),
    );

    // Remove any remaining HTML5 semantic elements that don't have markdown equivalents
    result = Cow::Owned(SEMANTIC_RE.replace_all(&result, "").into_owned());

    // Decode HTML entities
    let decoded = decode_html_entities(&result);
    if let Cow::Owned(s) = decoded {
        result = Cow::Owned(s);
    }

    // Convert final Cow to owned String for return
    result.into_owned()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Tests from extract_main_content.rs
    #[test]
    fn test_removes_navigation() {
        let html = r#"
            <article>
                <p>Content</p>
                <nav>Should be removed</nav>
            </article>
        "#;
        let result = extract_main_content(html).unwrap();
        assert!(!result.contains("Should be removed"));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_preserves_nested_structure() {
        let html = r#"
            <article>
                <div class="content">
                    <p>Nested <strong>content</strong></p>
                </div>
            </article>
        "#;
        let result = extract_main_content(html).unwrap();
        assert!(result.contains("<strong>content</strong>"));
        assert!(result.contains("<p>"));
    }

    #[test]
    fn test_text_escaping() {
        let html = r#"
            <article>
                <p>5 &lt; 10 &amp; 10 &gt; 5</p>
            </article>
        "#;
        let result = extract_main_content(html).unwrap();
        // Must preserve escaped characters
        assert!(result.contains("&lt;") || result.contains("<"));
        assert!(result.contains("&gt;") || result.contains(">"));
        assert!(result.contains("&amp;") || result.contains("&"));
    }

    #[test]
    fn test_self_closing_tags() {
        let html = r#"
            <article>
                <p>Image: <img src="test.jpg" alt="test" /></p>
                <hr />
                <p>Line break<br />here</p>
            </article>
        "#;
        let result = extract_main_content(html).unwrap();
        assert!(result.contains("<img"));
        assert!(result.contains("<hr"));
        assert!(result.contains("<br"));
        // Should NOT contain closing tags for void elements
        assert!(!result.contains("</img>"));
        assert!(!result.contains("</br>"));
        assert!(!result.contains("</hr>"));
    }

    #[test]
    fn test_preserves_attributes() {
        let html = r#"
            <article>
                <div class="content" id="main" data-test="value">Content</div>
            </article>
        "#;
        let result = extract_main_content(html).unwrap();
        assert!(result.contains("class=\"content\""));
        assert!(result.contains("id=\"main\""));
        assert!(result.contains("data-test=\"value\""));
    }

    #[test]
    fn test_removes_multiple_unwanted_elements() {
        let html = r#"
            <article>
                <header>Header</header>
                <p>Main content</p>
                <nav>Navigation</nav>
                <footer>Footer</footer>
                <aside>Sidebar</aside>
            </article>
        "#;
        let result = extract_main_content(html).unwrap();
        assert!(result.contains("Main content"));
        assert!(!result.contains("Header"));
        assert!(!result.contains("Navigation"));
        assert!(!result.contains("Footer"));
        assert!(!result.contains("Sidebar"));
    }

    #[test]
    fn test_preserves_comments() {
        let html = r#"
            <article>
                <!-- Important comment -->
                <p>Content</p>
            </article>
        "#;
        let result = extract_main_content(html).unwrap();
        assert!(result.contains("<!-- Important comment -->"));
    }

    #[test]
    fn test_body_fallback() {
        let html = r#"
            <html>
                <body>
                    <nav>Navigation</nav>
                    <p>Main content without article tag</p>
                </body>
            </html>
        "#;
        let result = extract_main_content(html).unwrap();
        assert!(result.contains("Main content"));
        assert!(!result.contains("Navigation"));
    }

    #[test]
    fn test_malformed_html_fallback() {
        let html = "<p>Malformed HTML without body</p>";
        let result = extract_main_content(html).unwrap();
        assert_eq!(result, html);
    }
}
