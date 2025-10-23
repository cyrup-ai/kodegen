//! Integration test for the search query engine
//! This tests the core search functionality with a simplified setup

use kodegen_tools_citescrape::search::SearchQueryType;

#[test]
fn test_query_type_parsing() {
    // Test text query
    let query = SearchQueryType::parse("simple text search");
    match query {
        SearchQueryType::Text(text) => assert_eq!(text, "simple text search"),
        _ => panic!("Expected text query"),
    }

    // Test phrase query
    let query = SearchQueryType::parse("\"exact phrase\"");
    match query {
        SearchQueryType::Phrase(phrase) => assert_eq!(phrase, "exact phrase"),
        _ => panic!("Expected phrase query"),
    }

    // Test field-specific query
    let query = SearchQueryType::parse("title:hello world");
    match query {
        SearchQueryType::Field { field, query } => {
            assert_eq!(field, "title");
            assert_eq!(query, "hello world");
        }
        _ => panic!("Expected field query"),
    }

    // Test fuzzy query with default distance
    let query = SearchQueryType::parse("test~");
    match query {
        SearchQueryType::Fuzzy { term, distance } => {
            assert_eq!(term, "test");
            assert_eq!(distance, 1);
        }
        _ => panic!("Expected fuzzy query"),
    }

    // Test fuzzy query with custom distance
    let query = SearchQueryType::parse("test~2");
    match query {
        SearchQueryType::Fuzzy { term, distance } => {
            assert_eq!(term, "test");
            assert_eq!(distance, 2);
        }
        _ => panic!("Expected fuzzy query"),
    }

    // Test boolean query
    let query = SearchQueryType::parse("hello AND world");
    match query {
        SearchQueryType::Boolean(boolean_str) => assert_eq!(boolean_str, "hello AND world"),
        _ => panic!("Expected boolean query"),
    }
}

#[test]
fn test_fuzzy_query_distance_limits() {
    // Test fuzzy query with distance over limit (should cap at 3)
    let query = SearchQueryType::parse("test~5");
    match query {
        SearchQueryType::Fuzzy { term, distance } => {
            assert_eq!(term, "test");
            assert_eq!(distance, 3); // Should be capped at 3
        }
        _ => panic!("Expected fuzzy query"),
    }

    // Test fuzzy query with invalid distance (should default to 1)
    let query = SearchQueryType::parse("test~invalid");
    match query {
        SearchQueryType::Fuzzy { term, distance } => {
            assert_eq!(term, "test");
            assert_eq!(distance, 1); // Should default to 1
        }
        _ => panic!("Expected fuzzy query"),
    }
}

#[test]
fn test_field_query_variations() {
    // Test different field names
    let test_cases = vec![
        ("content:hello", "content", "hello"),
        ("markdown:syntax", "markdown", "syntax"),
        ("url:example.com", "url", "example.com"),
        ("path:/docs/guide", "path", "/docs/guide"),
        ("title:multi word title", "title", "multi word title"),
    ];

    for (query_str, expected_field, expected_query) in test_cases {
        let query = SearchQueryType::parse(query_str);
        match query {
            SearchQueryType::Field { field, query } => {
                assert_eq!(field, expected_field);
                assert_eq!(query, expected_query);
            }
            _ => panic!("Expected field query for: {query_str}"),
        }
    }
}

#[test]
fn test_complex_query_patterns() {
    // Test edge cases and complex patterns

    // Empty phrase - this gets parsed as text since it doesn't match the phrase pattern
    let query = SearchQueryType::parse("\"\"");
    match query {
        SearchQueryType::Text(text) => assert_eq!(text, "\"\""),
        _ => panic!("Expected text query for empty phrase"),
    }

    // Single word phrase
    let query = SearchQueryType::parse("\"single\"");
    match query {
        SearchQueryType::Phrase(phrase) => assert_eq!(phrase, "single"),
        _ => panic!("Expected phrase query"),
    }

    // Field query with colon in value
    let query = SearchQueryType::parse("url:https://example.com:8080");
    match query {
        SearchQueryType::Field { field, query } => {
            assert_eq!(field, "url");
            assert_eq!(query, "https://example.com:8080");
        }
        _ => panic!("Expected field query"),
    }

    // Complex boolean expressions
    let boolean_queries = vec![
        "hello AND world",
        "rust OR python OR javascript",
        "hello AND (world OR universe)",
    ];

    for query_str in boolean_queries {
        let query = SearchQueryType::parse(query_str);
        match query {
            SearchQueryType::Boolean(boolean_str) => {
                assert_eq!(boolean_str, query_str);
            }
            _ => panic!("Expected boolean query for: {query_str}"),
        }
    }

    // Test NOT separately (it doesn't have spaces around it like " NOT ")
    let query = SearchQueryType::parse("NOT deprecated");
    match query {
        SearchQueryType::Text(text) => {
            assert_eq!(text, "NOT deprecated"); // This is parsed as text since it doesn't match " NOT "
        }
        _ => panic!("Expected text query for 'NOT deprecated'"),
    }

    // Test that field query takes precedence over boolean
    let query = SearchQueryType::parse("title:hello AND content:world");
    match query {
        SearchQueryType::Field { field, query } => {
            assert_eq!(field, "title");
            assert_eq!(query, "hello AND content:world");
        }
        _ => panic!("Expected field query (field takes precedence over boolean)"),
    }
}

#[test]
fn test_query_precedence() {
    // Test that query parsing follows correct precedence

    // Field query should take precedence over fuzzy
    let query = SearchQueryType::parse("title:test~2");
    match query {
        SearchQueryType::Field { field, query } => {
            assert_eq!(field, "title");
            assert_eq!(query, "test~2"); // The ~2 is part of the query value
        }
        _ => panic!("Expected field query"),
    }

    // Boolean should take precedence over simple text
    let query = SearchQueryType::parse("hello AND world test");
    match query {
        SearchQueryType::Boolean(_) => {}
        _ => panic!("Expected boolean query"),
    }

    // Phrase should take precedence when properly quoted
    let query = SearchQueryType::parse("\"hello AND world\"");
    match query {
        SearchQueryType::Phrase(phrase) => {
            assert_eq!(phrase, "hello AND world");
        }
        _ => panic!("Expected phrase query"),
    }
}
