//! Integration tests for the yyaml YAML parser library
//! 
//! This module organizes comprehensive integration tests by functional domain,
//! providing blazing-fast, zero-allocation testing of the complete YAML parsing pipeline.
//! 
//! ## Test Organization
//! 
//! - `alias_tests`: YAML alias and anchor resolution testing
//! - `structure_tests`: Complex YAML structure parsing (mappings, sequences, nesting)
//! - `lexer_tests`: Tokenization and scanning integration tests
//! - `parser_tests`: Event-driven parser state machine testing
//! - `serde_tests`: Serialization and deserialization integration tests
//! - `tag_tests`: YAML tag processing and type resolution testing
//! - `misc_tests`: Edge cases, verification, and miscellaneous integration tests
//! 
//! ## Performance Characteristics
//! 
//! All integration tests are designed for:
//! - Zero unnecessary allocations
//! - Blazing-fast execution with parallel testing
//! - Comprehensive error handling without unwrap()/expect() in production code
//! - Elegant, ergonomic test patterns
//! 
//! ## Usage with Nextest
//! 
//! ```bash
//! # Run all integration tests
//! cargo nextest run --profile integration
//! 
//! # Run specific test domain
//! cargo nextest run test(integration::alias)
//! cargo nextest run test(integration::parser)
//! cargo nextest run test(integration::serde)
//! ```

pub mod alias_tests;
pub mod structure_tests;
pub mod lexer_tests;
pub mod parser_tests;
pub mod serde_tests;
pub mod tag_tests;
pub mod misc_tests;

/// Common test utilities and fixtures for integration tests
pub mod common {
    use std::borrow::Cow;
    
    /// Test fixture for YAML parsing with zero-allocation string handling
    pub fn parse_yaml_fixture<'a>(yaml_content: &'a str) -> Result<crate::yaml::Yaml, Box<dyn std::error::Error + 'a>> {
        crate::parser::loader::YamlLoader::load_from_str(yaml_content)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            .and_then(|mut docs| {
                docs.into_iter().next()
                    .ok_or_else(|| "No YAML document found".into())
            })
    }
    
    /// High-performance assertion helper that avoids allocation in error paths
    pub fn assert_yaml_eq(actual: &crate::yaml::Yaml, expected: &crate::yaml::Yaml, context: &str) {
        if actual != expected {
            panic!(
                "YAML assertion failed in {context}\nExpected: {expected:?}\nActual: {actual:?}"
            );
        }
    }
    
    /// Zero-allocation string comparison for YAML scalar values
    pub fn assert_scalar_eq(yaml: &crate::yaml::Yaml, expected: &str, context: &str) {
        match yaml {
            crate::yaml::Yaml::String(s) => {
                if s != expected {
                    panic!("Scalar mismatch in {context}: expected '{expected}', got '{s}'");
                }
            },
            crate::yaml::Yaml::Integer(i) => {
                let expected_int = expected.parse::<i64>()
                    .unwrap_or_else(|_| panic!("Expected integer in {context}, got string: {expected}"));
                if *i != expected_int {
                    panic!("Integer mismatch in {context}: expected {expected_int}, got {i}");
                }
            },
            crate::yaml::Yaml::Real(r) => {
                let expected_real = expected.parse::<f64>()
                    .unwrap_or_else(|_| panic!("Expected real in {context}, got string: {expected}"));
                if (r - expected_real).abs() > f64::EPSILON {
                    panic!("Real mismatch in {context}: expected {expected_real}, got {r}");
                }
            },
            crate::yaml::Yaml::Boolean(b) => {
                let expected_bool = expected.parse::<bool>()
                    .unwrap_or_else(|_| panic!("Expected boolean in {context}, got string: {expected}"));
                if *b != expected_bool {
                    panic!("Boolean mismatch in {context}: expected {expected_bool}, got {b}");
                }
            },
            crate::yaml::Yaml::Null => {
                if expected != "null" && expected != "~" && expected != "" {
                    panic!("Null mismatch in {context}: expected '{expected}', got null");
                }
            },
            other => panic!("Unexpected YAML type in {context}: {other:?}"),
        }
    }
    
    /// Performance-optimized test setup that minimizes allocation overhead
    pub struct TestEnvironment {
        _private: (),
    }
    
    impl TestEnvironment {
        pub fn new() -> Self {
            Self { _private: () }
        }
        
        /// Initialize test environment with optimal settings
        pub fn setup() -> Self {
            // Initialize any global test state here
            Self::new()
        }
    }
    
    impl Default for TestEnvironment {
        fn default() -> Self {
            Self::new()
        }
    }
}