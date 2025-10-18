use kodegen_citescrape::content_saver::markdown_converter::html_preprocessing::*;

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
