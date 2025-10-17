use kodegen_citescrape::link_rewriter::LinkRewriter;

#[tokio::test]
async fn test_link_rewriter() {
    let rewriter = LinkRewriter::new("/output");
    
    // Register some URLs
    rewriter.register_url("https://example.com/", "/output/example.com/index.html").await;
    rewriter.register_url("https://example.com/about", "/output/example.com/about/index.html").await;
    
    let html = r#"<a href="/about">About</a>"#.to_string();
    let current_url = "https://example.com/".to_string();
    
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _task = rewriter.rewrite_links(html, current_url, move |result| {
        let _ = tx.send(result);
    });
    
    let result = rx.await.unwrap();
    let rewritten = match result {
        Ok(content) => content,
        Err(e) => panic!("Failed to rewrite links: {}", e),
    };
    
    assert!(rewritten.contains(r#"href="about/index.html""#));
}
