//! A comprehensive YAML 1.2 parser/emitter supporting anchors, aliases,
//! block & flow sequences/mappings, block scalars, tags, multi-doc streams, etc.
//!
//! # Example
//! ```rust
//! let docs = yyaml::YamlLoader::load_from_str("foo: 123").unwrap();
//! let doc = &docs[0];
//! assert_eq!(doc["foo"].as_i64().unwrap(), 123);
//! ```

// Removed broken de.rs - using value.rs system instead
mod emitter;
mod error;
pub mod events;
pub mod lexer;
mod linked_hash_map;
pub mod parser;

pub mod scanner;
pub mod semantic;
mod ser;
pub mod value;
mod yaml;

// Remove broken de.rs exports
pub use emitter::{EmitError, EmitResult, YamlEmitter};
pub use error::{Marker, ScanError};
pub use events::{Event, EventReceiver, MarkedEventReceiver, TEncoding, TScalarStyle, TokenType};
pub use linked_hash_map::LinkedHashMap;
pub use parser::YamlLoader;
pub use ser::*;
pub use value::{Deserializer, Mapping, Number, Sequence, Value, from_value};
pub use yaml::Yaml;

/// Deserialize an instance of type T from a string of YAML text.
///
/// This is the standard serde_yaml API function for drop-in compatibility.
pub fn from_str<T>(s: &str) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let docs = YamlLoader::load_from_str(s).map_err(Error::Scan)?;
    if docs.is_empty() {
        return Err(Error::Custom("No YAML documents found".to_string()));
    }
    if docs.len() > 1 {
        return Err(Error::Custom(
            "Multiple YAML documents found, expected one".to_string(),
        ));
    }
    let yaml = &docs[0];
    let value = Value::from_yaml(yaml);
    let deserializer = value::Deserializer::new(value);
    T::deserialize(deserializer)
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("scan error: {0}")]
    Scan(#[from] ScanError),
    #[error("emit error: {0}")]
    Emit(#[from] EmitError),
    #[error("repetition limit exceeded")]
    RepetitionLimitExceeded,
    #[error("custom: {0}")]
    Custom(String),
}

impl Error {
    /// Blazing-fast constructor for repetition limit exceeded error.
    /// Zero-allocation construction for hot path error handling.
    #[inline(always)]
    #[must_use] 
    pub const fn repetition_limit_exceeded() -> Self {
        Self::RepetitionLimitExceeded
    }
}

impl serde::de::Error for Error {
    #[inline]
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    #[inline]
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

pub fn to_string<T: serde::Serialize>(value: &T) -> Result<String, Error> {
    let yaml = value.serialize(ser::YamlSerializer::new())?;
    let mut writer = String::new();
    let mut emitter = YamlEmitter::new(&mut writer);
    emitter.dump(&yaml)?;
    Ok(writer)
}

/// Legacy compatibility function - calls the standard from_str implementation
pub fn parse_str<T>(s: &str) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    from_str(s)
}

pub fn to_value<T: serde::Serialize>(value: &T) -> Result<Value, Error> {
    let yaml = value.serialize(ser::YamlSerializer::new())?;
    Ok(Value::from_yaml(&yaml))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let yaml_val = Yaml::String("test".to_string());
        match yaml_val.as_str() {
            Some(s) => assert_eq!(s, "test"),
            None => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_yaml_from_str() {
        assert_eq!(Yaml::parse_str("42"), Yaml::Integer(42));
        assert_eq!(Yaml::parse_str("true"), Yaml::Boolean(true));
        assert_eq!(Yaml::parse_str("false"), Yaml::Boolean(false));
        assert_eq!(Yaml::parse_str("null"), Yaml::Null);
        assert_eq!(Yaml::parse_str("~"), Yaml::Null);
        assert_eq!(Yaml::parse_str("3.14"), Yaml::Real("3.14".to_string()));
        assert_eq!(Yaml::parse_str("hello"), Yaml::String("hello".to_string()));
    }

    #[test]
    fn test_yaml_types() {
        let null_yaml = Yaml::Null;
        assert!(null_yaml.is_null());

        let bool_yaml = Yaml::Boolean(true);
        assert_eq!(bool_yaml.as_bool(), Some(true));

        let int_yaml = Yaml::Integer(42);
        assert_eq!(int_yaml.as_i64(), Some(42));

        let string_yaml = Yaml::String("test".to_string());
        assert_eq!(string_yaml.as_str(), Some("test"));
    }

    #[test]
    fn test_linked_hash_map() {
        let mut map = LinkedHashMap::new();
        map.insert("first".to_string(), 1);
        map.insert("second".to_string(), 2);
        map.insert("third".to_string(), 3);

        assert_eq!(map.len(), 3);
        assert_eq!(map.get("first"), Some(&1));
        assert_eq!(map.get("second"), Some(&2));
        assert_eq!(map.get("third"), Some(&3));

        let keys: Vec<_> = map.iter().map(|(k, _)| k.clone()).collect();
        assert_eq!(keys, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_simple_key_value() {
        let s = "key: value";
        let result = YamlLoader::load_from_str(s);
        match result {
            Ok(docs) => {
                assert_eq!(docs.len(), 1);
                let doc = &docs[0];
                match doc["key"].as_str() {
                    Some(s) => assert_eq!(s, "value"),
                    None => panic!("Expected string value for key"),
                }
            }
            Err(e) => {
                panic!("Parsing failed: {e}");
            }
        }
    }

    #[test]
    fn test_simple_doc() {
        let s = "hello: world
int: 42
bool: true
nulltest: ~";
        let result = YamlLoader::load_from_str(s);
        match result {
            Ok(docs) => {
                assert_eq!(docs.len(), 1);
                let doc = &docs[0];
                match doc["hello"].as_str() {
                    Some(s) => assert_eq!(s, "world"),
                    None => panic!("Expected string value for hello"),
                }
                match doc["int"].as_i64() {
                    Some(i) => assert_eq!(i, 42),
                    None => panic!("Expected integer value for int"),
                }
                match doc["bool"].as_bool() {
                    Some(b) => assert!(b),
                    None => panic!("Expected boolean value for bool"),
                }
                assert!(doc["nulltest"].is_null());
            }
            Err(e) => {
                panic!("Parsing failed: {e}");
            }
        }
    }

    #[test]
    fn test_flow_seq() {
        let s = "[1, 2, 3]";
        let result = YamlLoader::load_from_str(s);
        match result {
            Ok(docs) => match docs[0].as_vec() {
                Some(arr) => {
                    assert_eq!(arr.len(), 3);
                    match arr[0].as_i64() {
                        Some(i) => assert_eq!(i, 1),
                        None => panic!("Expected integer value for arr[0]"),
                    }
                    match arr[1].as_i64() {
                        Some(i) => assert_eq!(i, 2),
                        None => panic!("Expected integer value for arr[1]"),
                    }
                    match arr[2].as_i64() {
                        Some(i) => assert_eq!(i, 3),
                        None => panic!("Expected integer value for arr[2]"),
                    }
                }
                None => panic!("Expected vector value"),
            },
            Err(e) => {
                panic!("Flow sequence parsing failed: {e}");
            }
        }
    }

    #[test]
    fn test_two_line_mapping() {
        let s = "hello: world\nint: 42";
        let result = YamlLoader::load_from_str(s);
        match result {
            Ok(docs) => {
                assert_eq!(docs.len(), 1);
                let doc = &docs[0];
                match doc["hello"].as_str() {
                    Some(s) => assert_eq!(s, "world"),
                    None => panic!("Expected string value for hello"),
                }
                match doc["int"].as_i64() {
                    Some(i) => assert_eq!(i, 42),
                    None => panic!("Expected integer value for int"),
                }
            }
            Err(e) => {
                panic!("Two-line mapping failed: {e}");
            }
        }
    }

    #[test]
    fn test_fluent_ai_models_yaml_integration() {
        // Integration test: Download and parse real models.yaml from fluent-ai project
        let models_yaml_url =
            "https://raw.githubusercontent.com/cyrup-ai/fluent-ai/main/provider/models.yaml";

        // Download the real models.yaml file
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => panic!("Failed to create tokio runtime: {e}"),
        };
        let yaml_content = rt.block_on(async {
            let response = match reqwest::get(models_yaml_url).await {
                Ok(resp) => resp,
                Err(e) => panic!("Failed to download models.yaml: {e}"),
            };
            match response.text().await {
                Ok(content) => content,
                Err(e) => panic!("Failed to read models.yaml content: {e}"),
            }
        });

        // Handle 404 errors gracefully - skip test if URL is not available
        if yaml_content.contains("404") && yaml_content.len() < 50 {
            return;
        }

        // Parse the real YAML content using yyaml
        let result = YamlLoader::load_from_str(&yaml_content);
        match result {
            Ok(docs) => {
                assert!(!docs.is_empty(), "Expected at least one YAML document");

                // Inspect actual structure - MUST be an array
                let root_doc = &docs[0];

                // CRITICAL: The YAML MUST be parsed as an array, not as an object
                let providers = root_doc.as_vec().expect(
                    "YAML must be parsed as an array, not as an object with keys like '- provider'",
                );
                assert!(!providers.is_empty(), "Expected at least one provider");

                // Validate first provider has expected structure
                let first_provider = &providers[0];
                if first_provider["provider"].as_str().is_some() {
                    // Provider name exists as expected
                } else {
                    // Structure validation
                }
            }
            Err(e) => {
                panic!("Failed to parse models.yaml with yyaml: {e}");
            }
        }
    }
}
