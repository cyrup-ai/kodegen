use kodegen_tools_citescrape::link_rewriter::LinkRewriter;

#[tokio::test]
async fn test_link_rewriter() {
    let rewriter = LinkRewriter::new("/output");

    // Register some URLs
    rewriter
        .register_url("https://example.com/", "/output/example.com/index.html")
        .await;
    rewriter
        .register_url(
            "https://example.com/about",
            "/output/example.com/about/index.html",
        )
        .await;

    let html = r#"<a href="/about">About</a>"#.to_string();
    let current_url = "https://example.com/".to_string();

    let rewritten = rewriter
        .rewrite_links(html, current_url)
        .await
        .expect("Failed to rewrite links");

    assert!(rewritten.contains(r#"href="about/index.html""#));
}
