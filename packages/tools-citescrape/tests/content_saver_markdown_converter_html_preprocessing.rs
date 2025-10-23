use kodegen_tools_citescrape::content_saver::markdown_converter::extract_main_content;

// Tests from extract_main_content.rs
#[test]
fn test_removes_navigation() {
    let html = r"
        <article>
            <p>Content</p>
            <nav>Should be removed</nav>
        </article>
    ";
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
    let html = r"
        <article>
            <p>5 &lt; 10 &amp; 10 &gt; 5</p>
        </article>
    ";
    let result = extract_main_content(html).unwrap();
    // Must preserve escaped characters
    assert!(result.contains("&lt;") || result.contains('<'));
    assert!(result.contains("&gt;") || result.contains('>'));
    assert!(result.contains("&amp;") || result.contains('&'));
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
    let html = r"
        <article>
            <header>Header</header>
            <p>Main content</p>
            <nav>Navigation</nav>
            <footer>Footer</footer>
            <aside>Sidebar</aside>
        </article>
    ";
    let result = extract_main_content(html).unwrap();
    assert!(result.contains("Main content"));
    assert!(!result.contains("Header"));
    assert!(!result.contains("Navigation"));
    assert!(!result.contains("Footer"));
    assert!(!result.contains("Sidebar"));
}

#[test]
fn test_preserves_comments() {
    let html = r"
        <article>
            <!-- Important comment -->
            <p>Content</p>
        </article>
    ";
    let result = extract_main_content(html).unwrap();
    assert!(result.contains("<!-- Important comment -->"));
}

#[test]
fn test_body_fallback() {
    let html = r"
        <html>
            <body>
                <nav>Navigation</nav>
                <p>Main content without article tag</p>
            </body>
        </html>
    ";
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
