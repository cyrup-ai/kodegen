//! Complete RFC Compliance Test Suite for YAML 1.2.2
//!
//! This module provides point-by-point test coverage for every requirement
//! in the YAML 1.2.2 specification, organized by chapter and section.
//!
//! ## Test Coverage
//!
//! ✅ **Chapter 5**: Character Productions (already complete)
//! ✅ **Chapter 6**: Structural Productions - indentation, separation, comments
//! ✅ **Chapter 7**: Flow Style Productions - scalars, collections, nodes
//! ✅ **Chapter 8**: Block Style Productions - scalars, collections, nodes
//! ✅ **Chapter 9**: Document Stream Productions - documents, streams
//! ✅ **Chapter 10**: Recommended Schemas - Failsafe, JSON, Core
//!
//! ## Production Rule Coverage
//!
//! Tests cover production rules [1-211] from the YAML 1.2.2 specification:
//! - Positive tests: Valid inputs that MUST parse correctly
//! - Negative tests: Invalid inputs that MUST be rejected
//! - Edge cases: Boundary conditions and corner cases
//! - Context tests: Context-dependent behavior verification
//!
//! ## Usage
//!
//! Run all RFC compliance tests:
//! ```bash
//! cargo test --test rfc_compliance
//! ```
//!
//! Run specific chapter tests:
//! ```bash
//! cargo test --test rfc_compliance ch06_structural_productions
//! ```

// Chapter modules - organized by YAML 1.2.2 specification structure
pub mod ch06_structural_productions;
pub mod ch07_flow_style_productions;
pub mod ch08_block_style_productions;
pub mod ch09_document_stream_productions;
pub mod ch10_schemas;

/// Comprehensive spec example tests from YAML 1.2.2 documentation
#[cfg(test)]
mod spec_examples {
    use yyaml::YamlLoader;
    
    /// Test Example 2.1: Sequence of Scalars
    #[test]
    fn test_spec_example_2_1_sequence_of_scalars() {
        let yaml = r#"
- Mark McGwire
- Sammy Sosa  
- Ken Griffey
"#;
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        assert_eq!(docs[0].as_vec().unwrap().len(), 3);
        assert_eq!(docs[0][0].as_str().unwrap(), "Mark McGwire");
        assert_eq!(docs[0][1].as_str().unwrap(), "Sammy Sosa");
        assert_eq!(docs[0][2].as_str().unwrap(), "Ken Griffey");
    }
    
    /// Test Example 2.2: Mapping Scalars to Scalars
    #[test]
    fn test_spec_example_2_2_mapping_scalars() {
        let yaml = r#"
hr:  65    # Home runs
avg: 0.278 # Batting average
rbi: 147   # Runs Batted In
"#;
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        assert_eq!(docs[0]["hr"].as_i64().unwrap(), 65);
        assert!((docs[0]["avg"].as_f64().unwrap() - 0.278).abs() < f64::EPSILON);
        assert_eq!(docs[0]["rbi"].as_i64().unwrap(), 147);
    }    
    /// Test Example 2.3: Mapping Scalars to Sequences
    #[test]
    fn test_spec_example_2_3_mapping_to_sequences() {
        let yaml = r#"
american:
  - Boston Red Sox
  - Detroit Tigers
  - New York Yankees
national:
  - New York Mets
  - Chicago Cubs
  - Atlanta Braves
"#;
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        assert_eq!(docs[0]["american"].as_vec().unwrap().len(), 3);
        assert_eq!(docs[0]["national"].as_vec().unwrap().len(), 3);
        assert_eq!(docs[0]["american"][0].as_str().unwrap(), "Boston Red Sox");
    }
    
    /// Test Example 2.4: Sequence of Mappings
    #[test]
    fn test_spec_example_2_4_sequence_of_mappings() {
        let yaml = r#"
-
  name: Mark McGwire
  hr:   65
  avg:  0.278
-
  name: Sammy Sosa
  hr:   63
  avg:  0.288
"#;
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        let seq = docs[0].as_vec().unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0]["name"].as_str().unwrap(), "Mark McGwire");
        assert_eq!(seq[1]["name"].as_str().unwrap(), "Sammy Sosa");
    }
    
    /// Test Example 2.5: Sequence of Sequences
    #[test]
    fn test_spec_example_2_5_sequence_of_sequences() {
        let yaml = r#"
- [name        , hr, avg  ]
- [Mark McGwire, 65, 0.278]
- [Sammy Sosa  , 63, 0.288]
"#;
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        let outer_seq = docs[0].as_vec().unwrap();
        assert_eq!(outer_seq.len(), 3);
        assert_eq!(outer_seq[0].as_vec().unwrap().len(), 3);
        assert_eq!(outer_seq[1][0].as_str().unwrap(), "Mark McGwire");
    }
    
    /// Test Example 2.6: Mapping of Mappings
    #[test]
    fn test_spec_example_2_6_mapping_of_mappings() {
        let yaml = r#"
Mark McGwire: {hr: 65, avg: 0.278}
Sammy Sosa: {
    hr: 63,
    avg: 0.288
  }
"#;
        let docs = YamlLoader::load_from_str(yaml).unwrap();
        assert_eq!(docs[0]["Mark McGwire"]["hr"].as_i64().unwrap(), 65);
        assert_eq!(docs[0]["Sammy Sosa"]["hr"].as_i64().unwrap(), 63);
    }
}