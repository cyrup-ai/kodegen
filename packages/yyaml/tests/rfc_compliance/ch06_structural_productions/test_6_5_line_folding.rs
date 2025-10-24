//! RFC 6.5 Line Folding Compliance Tests
//!
//! Tests production rules [73-74] for line folding
//! References: ../../../docs/ch06-structural-productions/6.5-line-folding.md

use yyaml::YamlLoader;

/// Test [73] s-separate-in-line - line folding in flow context
#[test]
fn test_production_73_line_folding() {
    // Line folding should convert line breaks to spaces
    let yaml = r#"
text: >
  This is a long line
  that should be folded
  into a single line.
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let result = docs[0]["text"].as_str().unwrap();
    assert!(result.contains("This is a long line that should be folded into a single line."));
}

/// Test [74] b-l-folded(n,c) - folded line production
#[test]
fn test_production_74_folded_lines() {
    let yaml = r#"
folded: >
  Line 1
  Line 2

  Line 4 after blank
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    let result = docs[0]["folded"].as_str().unwrap();
    // Folded style should preserve double line breaks but fold single ones
    assert!(result.contains("Line 1 Line 2"));
    assert!(result.contains("\n\nLine 4"));
}

/// Test folding in different contexts
#[test]
fn test_context_dependent_folding() {
    // Flow scalar with line folding
    let yaml = r#"
"Multi-line
string in flow"
"#;
    let docs = YamlLoader::load_from_str(yaml).unwrap();
    assert_eq!(docs[0].as_str().unwrap(), "Multi-line string in flow");
}