//! RFC Compliance Integration Tests
//!
//! Main integration test file for YAML 1.2.2 RFC compliance.
//! This file includes all chapter test modules and provides comprehensive
//! validation of the yyaml library against the YAML specification.
//!
//! ## Test Organization
//!
//! - **Chapter 6**: Structural Productions (indentation, separation, comments)
//! - **Chapter 7**: Flow Style Productions (scalars, collections, nodes)
//! - **Chapter 8**: Block Style Productions (scalars, collections, nodes)
//! - **Chapter 9**: Document Stream Productions (documents, streams)
//! - **Chapter 10**: Recommended Schemas (Failsafe, JSON, Core)
//!
//! ## Usage
//!
//! Run all RFC compliance tests:
//! ```bash
//! cargo test --test test_rfc_compliance
//! ```
//!
//! Run with verbose output:
//! ```bash
//! cargo test --test test_rfc_compliance -- --nocapture
//! ```
//!
//! Run specific test pattern:
//! ```bash
//! cargo test --test test_rfc_compliance test_production_63
//! ```

use yyaml::YamlLoader;

// Include all RFC compliance test modules
mod rfc_compliance;

/// Integration test: Verify all chapters work together
#[test]
fn test_comprehensive_yaml_document() {
    let yaml = r#"%YAML 1.2
%TAG ! tag:example.com,2000:app/
---
# Document with all YAML features
metadata:
  title: "Comprehensive YAML Test"
  version: 1.2
  authors: 
    - Alice
    - Bob
  
# Structural productions (Chapter 6)
configuration:
  # Comments and indentation
  database:
    host: localhost
    port: 5432
    credentials: &creds
      username: admin
      password: secret
      
  # Flow style productions (Chapter 7)
  features: [auth, logging, caching]
  flags: { debug: true, verbose: false, trace: on }
  
  # Block style productions (Chapter 8)
  description: |
    This is a literal block scalar
    that preserves line breaks
    and indentation.
    
  folded_text: >
    This is a folded block scalar
    that joins lines with spaces
    but preserves paragraph breaks.
    
  # Schemas (Chapter 10)  
  types:
    null_value: null
    boolean_true: yes
    boolean_false: no
    integer: 42
    float: 3.14159
    scientific: 1.23e-4
    hex: 0xFF
    octal: 0o77
    binary: 0b1010
    infinity: .inf
    not_a_number: .nan
    
# Aliases and references
production: 
  <<: *creds
  host: prod.example.com
  
servers:
  - name: web1
    config: *creds
  - name: web2  
    config: *creds
...
# Document stream productions (Chapter 9)
---
second_document: "This demonstrates multi-document streams"
..."#;
    
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    
    // Verify we parsed multiple documents
    assert_eq!(docs.len(), 2);
    
    // Test first document structure
    let doc = &docs[0];
    assert_eq!(doc["metadata"]["title"].as_str().unwrap(), "Comprehensive YAML Test");
    assert_eq!(doc["metadata"]["version"].as_f64().unwrap(), 1.2);
    assert_eq!(doc["metadata"]["authors"].as_vec().unwrap().len(), 2);
    
    // Test configuration structure
    assert_eq!(doc["configuration"]["database"]["port"].as_i64().unwrap(), 5432);
    assert_eq!(doc["configuration"]["features"].as_vec().unwrap().len(), 3);
    assert_eq!(doc["configuration"]["flags"]["debug"].as_bool().unwrap(), true);
    
    // Test block scalars
    assert!(doc["configuration"]["description"].as_str().unwrap().contains("literal block scalar"));
    assert!(doc["configuration"]["folded_text"].as_str().unwrap().contains("folded block scalar"));
    
    // Test type resolution
    assert!(doc["configuration"]["types"]["null_value"].is_null());
    assert_eq!(doc["configuration"]["types"]["boolean_true"].as_bool().unwrap(), true);
    assert_eq!(doc["configuration"]["types"]["integer"].as_i64().unwrap(), 42);
    
    // Test aliases work correctly
    assert_eq!(doc["production"]["username"].as_str().unwrap(), "admin");
    assert_eq!(doc["servers"][0]["config"]["username"].as_str().unwrap(), "admin");
    
    // Test second document
    assert_eq!(docs[1]["second_document"].as_str().unwrap(), "This demonstrates multi-document streams");
}/// Integration test: Error handling and edge cases
#[test]
fn test_error_handling_compliance() {
    // Test cases that should fail parsing
    let invalid_cases = vec![
        // Invalid indentation (tabs)
        "key:\n\tvalue",
        
        // Invalid flow syntax
        "[unclosed sequence",
        "{unclosed: mapping",
        
        // Invalid escape sequences in double quotes
        "\"invalid \\x escape\"",
        
        // Invalid document markers
        "content\n---\n... extra content after end",
    ];
    
    for (i, yaml) in invalid_cases.iter().enumerate() {
        let result = YamlLoader::load_from_str(yaml);
        assert!(
            result.is_err(),
            "Case {} should fail but parsed successfully: {:?}",
            i,
            yaml
        );
    }
}

/// Integration test: Performance and large document handling
#[test]
fn test_large_document_performance() {
    // Generate a reasonably large but structured YAML document
    let mut yaml = String::from("large_data:\n");
    
    // Add 1000 items to test scalability
    for i in 0..1000 {
        yaml.push_str(&format!("  item_{}: value_{}\n", i, i));
    }
    
    // Should parse without issues
    let docs = YamlLoader::load_from_str(&yaml).unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["large_data"]["item_0"].as_str().unwrap(), "value_0");
    assert_eq!(docs[0]["large_data"]["item_999"].as_str().unwrap(), "value_999");
}

/// Integration test: Unicode and international content
#[test]
fn test_unicode_compliance() {
    let yaml = r#"
unicode_content:
  chinese: "你好世界"
  japanese: "こんにちは世界"
  emoji: "🌍🚀✨"
  mathematical: "∫∑∞≠≤≥"
  arrows: "←→↑↓⇒⇔"
  
# BOM handling
mixed_scripts:
  - العربية
  - Русский  
  - Ελληνικά
  - हिन्दी
"#;
    
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0]["unicode_content"]["chinese"].as_str().unwrap(), "你好世界");
    assert_eq!(docs[0]["unicode_content"]["emoji"].as_str().unwrap(), "🌍🚀✨");
    assert_eq!(docs[0]["mixed_scripts"].as_vec().unwrap().len(), 4);
}

/// Integration test: All production rules work together
#[test]
fn test_production_rules_integration() {
    // This test combines multiple production rules to ensure they work together
    let yaml = r#"
# [75-79] Comments with [63-65] indentation
complex_structure: # End-of-line comment
  # [67-69] Line prefixes with [80-81] separation
  sequence_with_mappings:
    - # [107-116] Double-quoted scalars in [136-150] flow collections
      name: "Item \"One\""
      values: [1, 2, 3]
      
    - # [126-135] Plain scalars with [202-204] document structure
      name: Item Two
      description: >
        This is a folded scalar
        that demonstrates line folding
        according to production rules.
        
      data: |
        Literal scalar preserving
        exact formatting including
            indentation.
            
  # [151-162] Block collections with [183-199] node properties  
  anchored_data: &shared_config
    database:
      host: localhost
      port: 5432
      
  references:
    production: *shared_config
    staging: 
      <<: *shared_config
      host: staging.example.com
"#;
    
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let doc = &docs[0];
    
    // Verify complex structure parsed correctly
    assert_eq!(doc["complex_structure"]["sequence_with_mappings"].as_vec().unwrap().len(), 2);
    assert_eq!(doc["complex_structure"]["sequence_with_mappings"][0]["name"].as_str().unwrap(), "Item \"One\"");
    
    // Verify block scalars
    let folded = doc["complex_structure"]["sequence_with_mappings"][1]["description"].as_str().unwrap();
    assert!(folded.contains("folded scalar"));
    
    let literal = doc["complex_structure"]["sequence_with_mappings"][1]["data"].as_str().unwrap();
    assert!(literal.contains("    indentation"));
    
    // Verify anchors and aliases
    assert_eq!(doc["complex_structure"]["anchored_data"]["database"]["port"].as_i64().unwrap(), 5432);
    assert_eq!(doc["complex_structure"]["references"]["production"]["database"]["port"].as_i64().unwrap(), 5432);
    assert_eq!(doc["complex_structure"]["references"]["staging"]["host"].as_str().unwrap(), "staging.example.com");
}