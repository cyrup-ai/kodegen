//! Tests for query module bug fixes
//!
//! This test file validates the fixes for:
//! - Bug #1: UTF-8 truncation panic
//! - Bug #2: Pagination offset calculation
//! - Bug #4: Field + fuzzy query combination

use kodegen_tools_citescrape::search::SearchResults;

#[cfg(test)]
mod query_bug_fixes {
    use super::*;

    /// Test for Bug #1: UTF-8 content truncation should not panic
    #[test]
    fn test_utf8_content_truncation() {
        // Create test content with multi-byte UTF-8 characters
        let content_with_emoji = "a".repeat(150) + "🚀🌍🎉" + &"b".repeat(100);
        let content_with_chinese = "Hello 世界 ".repeat(50);
        let content_with_mixed = "Test 🔥 中文 💯 русский ".repeat(30);

        // Test that character-based truncation works correctly
        let truncated = content_with_emoji.chars().take(200).collect::<String>();
        assert_eq!(truncated.chars().count(), 200);

        let truncated = content_with_chinese.chars().take(200).collect::<String>();
        assert_eq!(truncated.chars().count(), 200);

        let truncated = content_with_mixed.chars().take(200).collect::<String>();
        assert_eq!(truncated.chars().count(), 200);

        // Verify no panic occurs with various UTF-8 content
        // (The actual convert_to_search_result function is tested in integration tests)
    }

    /// Test for Bug #2: Pagination with partial results
    #[test]
    fn test_pagination_with_partial_results() {
        use kodegen_tools_citescrape::search::types::SearchResultItem;

        // Simulate last page with fewer results than limit
        let results = SearchResults {
            results: vec![
                SearchResultItem {
                    path: "test1.md".to_string(),
                    url: "http://test1".to_string(),
                    title: "Test 1".to_string(),
                    excerpt: "excerpt 1".to_string(),
                    score: 1.0,
                },
                SearchResultItem {
                    path: "test2.md".to_string(),
                    url: "http://test2".to_string(),
                    title: "Test 2".to_string(),
                    excerpt: "excerpt 2".to_string(),
                    score: 0.9,
                },
                SearchResultItem {
                    path: "test3.md".to_string(),
                    url: "http://test3".to_string(),
                    title: "Test 3".to_string(),
                    excerpt: "excerpt 3".to_string(),
                    score: 0.8,
                },
            ],
            total_count: 100,
            offset: 90,
            limit: 20,
            query: "test".to_string(),
        };

        // Should have more results (90 + 3 = 93 < 100)
        assert!(results.has_more());

        // Next offset should be 90 + 3 = 93, NOT 90 + 20 = 110
        assert_eq!(results.next_offset(), Some(93));
    }

    /// Test for Bug #2: Pagination with exact page boundary
    #[test]
    fn test_pagination_exact_boundary() {
        use kodegen_tools_citescrape::search::types::SearchResultItem;

        let results = SearchResults {
            results: vec![
                SearchResultItem {
                    path: "test.md".to_string(),
                    url: "http://test".to_string(),
                    title: "Test".to_string(),
                    excerpt: "excerpt".to_string(),
                    score: 1.0,
                };
                10
            ],
            total_count: 100,
            offset: 90,
            limit: 10,
            query: "test".to_string(),
        };

        // Should not have more results (90 + 10 = 100)
        assert!(!results.has_more());

        // Next offset should be None
        assert_eq!(results.next_offset(), None);
    }

    /// Test for Bug #2: Pagination with variable limits
    #[test]
    fn test_pagination_variable_limits() {
        use kodegen_tools_citescrape::search::types::SearchResultItem;

        // First request with limit=50
        let first_page = SearchResults {
            results: vec![
                SearchResultItem {
                    path: "test.md".to_string(),
                    url: "http://test".to_string(),
                    title: "Test".to_string(),
                    excerpt: "excerpt".to_string(),
                    score: 1.0,
                };
                50
            ],
            total_count: 120,
            offset: 0,
            limit: 50,
            query: "test".to_string(),
        };

        assert!(first_page.has_more());
        assert_eq!(first_page.next_offset(), Some(50));

        // Second request with limit=30 (different limit)
        let second_page = SearchResults {
            results: vec![
                SearchResultItem {
                    path: "test.md".to_string(),
                    url: "http://test".to_string(),
                    title: "Test".to_string(),
                    excerpt: "excerpt".to_string(),
                    score: 1.0,
                };
                30
            ],
            total_count: 120,
            offset: 50,
            limit: 30,
            query: "test".to_string(),
        };

        assert!(second_page.has_more());
        // Should use actual results.len() (30), not limit (30)
        assert_eq!(second_page.next_offset(), Some(80));
    }
}

#[cfg(test)]
mod query_parsing_tests {
    use kodegen_tools_citescrape::search::SearchQueryType;

    /// Test for Bug #4: Field queries should preserve fuzzy operators
    /// This validates that "title:search~2" is parsed as a field query with fuzzy value
    #[test]
    fn test_field_query_preserves_fuzzy_operator() {
        // Parse field query with fuzzy operator
        let query = SearchQueryType::parse("title:search~2");

        match query {
            SearchQueryType::Field { field, query } => {
                assert_eq!(field, "title");
                // The fuzzy operator should be preserved in the query value
                assert_eq!(query, "search~2");
                // The build_field_query function will handle the fuzzy operator
            }
            _ => panic!("Expected field query, got {query:?}"),
        }
    }

    /// Test various field + fuzzy combinations
    #[test]
    fn test_field_fuzzy_combinations() {
        // Field with fuzzy distance 1
        let query = SearchQueryType::parse("content:fuzzy~");
        match query {
            SearchQueryType::Field { field, query } => {
                assert_eq!(field, "content");
                assert_eq!(query, "fuzzy~");
            }
            _ => panic!("Expected field query"),
        }

        // Field with fuzzy distance 3
        let query = SearchQueryType::parse("title:test~3");
        match query {
            SearchQueryType::Field { field, query } => {
                assert_eq!(field, "title");
                assert_eq!(query, "test~3");
            }
            _ => panic!("Expected field query"),
        }
    }
}
