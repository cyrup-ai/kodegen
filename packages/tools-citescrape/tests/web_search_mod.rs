use kodegen_tools_citescrape::web_search::*;

#[tokio::test]
#[ignore] // Requires browser installation
async fn test_search_basic() {
    let results = search("rust programming").await.unwrap();
    assert!(!results.results.is_empty());
    assert_eq!(results.query, "rust programming");
}
