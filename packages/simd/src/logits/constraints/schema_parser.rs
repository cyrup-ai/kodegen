//! JSON Schema to regex conversion based on outlines-core approach
//!
//! This module implements comprehensive JSON schema parsing that converts JSON schemas
//! to regular expressions for constrained generation. Based on the proven outlines-core
//! implementation with zero-allocation optimizations.

use anyhow::{Context, Result as AnyResult};
use regex::escape;
use rustc_hash::FxHashSet;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Default whitespace pattern for JSON
const WHITESPACE: &str = r"[ \t\n\r]*";

/// Maximum recursion depth to prevent exponential regex growth
const DEFAULT_MAX_RECURSION_DEPTH: usize = 3;

/// Core JSON schema parser that converts schemas to regex patterns
#[derive(Debug)]
pub struct SchemaParser<'a> {
    /// Reference to the root schema document
    root: &'a Value,
    /// Whitespace pattern to use in generated regex
    whitespace_pattern: &'a str,
    /// Current recursion depth
    recursion_depth: usize,
    /// Maximum allowed recursion depth
    max_recursion_depth: usize,
}

impl<'a> SchemaParser<'a> {
    /// Create a new schema parser with default settings
    #[inline]
    pub fn new(root: &'a Value) -> Self {
        Self {
            root,
            whitespace_pattern: WHITESPACE,
            recursion_depth: 0,
            max_recursion_depth: DEFAULT_MAX_RECURSION_DEPTH,
        }
    }

    /// Set custom whitespace pattern
    #[inline]
    pub fn with_whitespace_pattern(self, whitespace_pattern: &'a str) -> Self {
        Self {
            whitespace_pattern,
            ..self
        }
    }

    /// Set maximum recursion depth
    #[inline]
    pub fn with_max_recursion_depth(self, max_recursion_depth: usize) -> Self {
        Self {
            max_recursion_depth,
            ..self
        }
    }

    /// Convert JSON schema to regex pattern
    pub fn to_regex(&mut self, json: &Value) -> AnyResult<String> {
        // Prevent infinite recursion and exponential regex growth
        if self.recursion_depth >= self.max_recursion_depth {
            return Ok(r".*".to_string());
        }

        match json {
            Value::Object(obj) if obj.is_empty() => self.parse_empty_object(),
            Value::Object(obj) if obj.contains_key("properties") => self.parse_properties(obj),
            Value::Object(obj) if obj.contains_key("allOf") => self.parse_all_of(obj),
            Value::Object(obj) if obj.contains_key("anyOf") => self.parse_any_of(obj),
            Value::Object(obj) if obj.contains_key("oneOf") => self.parse_one_of(obj),
            Value::Object(obj) if obj.contains_key("prefixItems") => self.parse_prefix_items(obj),
            Value::Object(obj) if obj.contains_key("items") => self.parse_array_with_items(obj),
            Value::Object(obj) if obj.contains_key("enum") => self.parse_enum(obj),
            Value::Object(obj) if obj.contains_key("const") => self.parse_const(obj),
            Value::Object(obj) if obj.contains_key("$ref") => self.parse_ref(obj),
            Value::Object(obj) if obj.contains_key("type") => self.parse_type(obj),
            _ => Ok(r".*".to_string()),
        }
    }

    /// Handle empty schema objects (any JSON type allowed)
    fn parse_empty_object(&mut self) -> AnyResult<String> {
        let types = [
            json!({"type": "boolean"}),
            json!({"type": "null"}),
            json!({"type": "number"}),
            json!({"type": "integer"}),
            json!({"type": "string"}),
            json!({"type": "array"}),
            json!({"type": "object"}),
        ];

        let mut patterns = Vec::with_capacity(types.len());
        for object in types.iter() {
            let pattern = self.to_regex(object)?;
            patterns.push(format!("({pattern})"));
        }
        Ok(patterns.join("|"))
    }

    /// Parse object schemas with properties
    fn parse_properties(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let properties = obj
            .get("properties")
            .and_then(Value::as_object)
            .context("Properties field not found or invalid")?;

        let required_properties = obj
            .get("required")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .collect::<FxHashSet<_>>()
            })
            .unwrap_or_default();

        let additional_properties = obj.get("additionalProperties").unwrap_or(&json!(true));

        let min_properties = obj
            .get("minProperties")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;

        let max_properties = obj
            .get("maxProperties")
            .and_then(Value::as_u64)
            .map(|v| v as usize);

        self.recursion_depth += 1;
        let result = self.build_object_regex(
            properties,
            &required_properties,
            additional_properties,
            min_properties,
            max_properties,
        );
        self.recursion_depth -= 1;
        result
    }

    /// Build regex for object with properties constraints
    #[allow(clippy::too_many_arguments)]
    fn build_object_regex(
        &mut self,
        properties: &serde_json::Map<String, Value>,
        required: &FxHashSet<&str>,
        additional_properties: &Value,
        min_properties: usize,
        max_properties: Option<usize>,
    ) -> AnyResult<String> {
        let ws = self.whitespace_pattern;

        // Handle empty object case
        if properties.is_empty() && required.is_empty() && min_properties == 0 {
            return match additional_properties {
                Value::Bool(false) => Ok(format!(r"\{{{ws}\}}")),
                _ => Ok(format!(r"\{{{ws}.*{ws}\}}")),
            };
        }

        let mut regex = format!(r"\{{{ws}");

        // Collect required and optional properties
        let mut required_patterns = Vec::new();
        let mut optional_patterns = Vec::new();

        for (prop_name, prop_schema) in properties {
            let prop_regex = self.to_regex(prop_schema)?;
            let prop_pattern = format!(r#""{}"{}:{}{}"#, escape(prop_name), ws, ws, prop_regex);

            if required.contains(prop_name.as_str()) {
                required_patterns.push(prop_pattern);
            } else {
                optional_patterns.push(prop_pattern);
            }
        }

        // Build property combinations
        if !required_patterns.is_empty() {
            regex.push_str(&required_patterns.join(&format!("{ws},{ws}")));

            if !optional_patterns.is_empty() {
                // Add optional properties with proper combinations
                let max_optional = max_properties
                    .map(|max| max.saturating_sub(required_patterns.len()))
                    .unwrap_or(optional_patterns.len());

                if max_optional > 0 {
                    let optional_count = std::cmp::min(max_optional, optional_patterns.len());
                    regex.push_str(&format!(
                        "({},{},{})?",
                        ws,
                        ws,
                        Self::generate_property_combinations(
                            &optional_patterns,
                            optional_count,
                            ws
                        )
                    ));
                }
            }
        } else if !optional_patterns.is_empty() {
            // Only optional properties
            let max_count = max_properties.unwrap_or(optional_patterns.len());
            let min_count = std::cmp::max(min_properties, 0);

            if min_count == 0 {
                regex.push_str(&format!(
                    "({})?",
                    Self::generate_property_combinations(&optional_patterns, max_count, ws)
                ));
            } else {
                // Need at least min_count properties
                regex.push_str(&Self::generate_property_combinations(
                    &optional_patterns,
                    max_count,
                    ws,
                ));
            }
        }

        // Handle additional properties
        if !matches!(additional_properties, Value::Bool(false)) {
            let additional_regex = match additional_properties {
                Value::Bool(true) => format!(r#""[^"]*"{ws}:{ws}[^,}}]*"#),
                schema => {
                    let schema_regex = self.to_regex(schema)?;
                    format!(r#""[^"]*"{ws}:{ws}{schema_regex}"#)
                }
            };

            if properties.is_empty() && required.is_empty() {
                regex.push_str(&format!(
                    "({additional_regex}({ws},{ws},{additional_regex})*)?"
                ));
            } else {
                regex.push_str(&format!("({ws},{ws},{additional_regex})*"));
            }
        }

        regex.push_str(&format!(r"{ws}\}}"));
        Ok(regex)
    }

    /// Generate combinations of optional properties
    fn generate_property_combinations(patterns: &[String], max_count: usize, ws: &str) -> String {
        if patterns.is_empty() || max_count == 0 {
            return String::new();
        }

        if max_count == 1 {
            return patterns.join(&format!("|{ws}"));
        }

        // For multiple properties, create alternation with comma separation
        let mut combinations = Vec::new();

        // Single property options
        for pattern in patterns {
            combinations.push(pattern.clone());
        }

        // Multiple property combinations (simplified for performance)
        if max_count > 1 && patterns.len() > 1 {
            for (i, pattern1) in patterns.iter().enumerate() {
                for pattern2 in patterns.iter().skip(i + 1) {
                    combinations.push(format!("{}{ws},{ws}{}", pattern1, pattern2));
                }
            }
        }

        combinations.join(&format!("|{ws}"))
    }

    /// Parse type constraints
    fn parse_type(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let type_value = obj.get("type").context("Type field missing")?;

        match type_value {
            Value::String(type_str) => self.generate_type_regex(type_str.as_str(), obj),
            Value::Array(types) => {
                let mut patterns = Vec::with_capacity(types.len());
                for type_val in types {
                    if let Value::String(type_str) = type_val {
                        let pattern = self.generate_type_regex(type_str.as_str(), obj)?;
                        patterns.push(format!("({pattern})"));
                    }
                }
                Ok(patterns.join("|"))
            }
            _ => Ok(r".*".to_string()),
        }
    }

    /// Generate regex for specific JSON type
    #[inline]
    fn generate_type_regex(
        &mut self,
        type_str: &str,
        obj: &serde_json::Map<String, Value>,
    ) -> AnyResult<String> {
        match type_str {
            "null" => Ok("null".to_string()),
            "boolean" => Ok("(true|false)".to_string()),
            "integer" => self.generate_integer_regex(obj),
            "number" => self.generate_number_regex(obj),
            "string" => self.generate_string_regex(obj),
            "array" => self.generate_array_regex(obj),
            "object" => self.generate_object_regex(obj),
            _ => Ok(r".*".to_string()),
        }
    }

    /// Generate regex for integer type with constraints
    fn generate_integer_regex(&self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let minimum = obj.get("minimum").and_then(Value::as_i64);
        let maximum = obj.get("maximum").and_then(Value::as_i64);
        let multiple_of = obj.get("multipleOf").and_then(Value::as_u64);

        match (minimum, maximum, multiple_of) {
            (Some(min), Some(max), None) if min >= 0 && max <= 9999 => {
                // Generate specific range regex for small positive ranges
                let range_values = (min..=max)
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join("|");
                Ok(format!(r"({range_values})"))
            }
            (Some(0), None, None) => Ok(r"(0|[1-9][0-9]*)".to_string()),
            (Some(min), None, None) if min > 0 => Ok(r"[1-9][0-9]*".to_string()),
            _ => Ok(r"-?(0|[1-9][0-9]*)".to_string()),
        }
    }

    /// Generate regex for number type with constraints
    fn generate_number_regex(&self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let minimum = obj.get("minimum").and_then(Value::as_f64);
        let maximum = obj.get("maximum").and_then(Value::as_f64);

        match (minimum, maximum) {
            (Some(min), Some(_max)) if min >= 0.0 => {
                // Non-negative numbers
                Ok(r"(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?".to_string())
            }
            _ => Ok(r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?".to_string()),
        }
    }

    /// Generate regex for string type with constraints
    fn generate_string_regex(&self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let min_length = obj.get("minLength").and_then(Value::as_u64).unwrap_or(0);
        let max_length = obj.get("maxLength").and_then(Value::as_u64);
        let pattern = obj.get("pattern").and_then(Value::as_str);

        if let Some(custom_pattern) = pattern {
            Ok(format!(r#""{custom_pattern}""#))
        } else {
            let content_regex = match (min_length, max_length) {
                (0, None) => r#"[^"]*"#.to_string(),
                (min, None) => format!(r#"[^"]{{{min},}}"#),
                (0, Some(max)) => format!(r#"[^"]{{0,{max}}}"#),
                (min, Some(max)) => format!(r#"[^"]{{{min},{max}}}"#),
            };
            Ok(format!(r#""{content_regex}""#))
        }
    }

    /// Generate regex for array type with constraints
    fn generate_array_regex(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let ws = self.whitespace_pattern;
        let min_items = obj.get("minItems").and_then(Value::as_u64).unwrap_or(0);
        let max_items = obj.get("maxItems").and_then(Value::as_u64);

        let item_regex = if let Some(items) = obj.get("items") {
            self.to_regex(items)?
        } else if let Some(_prefix_items) = obj.get("prefixItems") {
            return self.parse_prefix_items(obj);
        } else {
            r"[^,\[\]]*".to_string()
        };

        match (min_items, max_items) {
            (0, None) => Ok(format!(
                r"\[{ws}({item_regex}({ws},{ws}{item_regex})*)?{ws}\]"
            )),
            (0, Some(0)) => Ok(format!(r"\[{ws}\]")),
            (min, None) => {
                if min == 1 {
                    Ok(format!(r"\[{ws}{item_regex}({ws},{ws}{item_regex})*{ws}\]"))
                } else {
                    let required_part = format!(
                        "{item_regex}({ws},{ws}{item_regex}){{{}}}",
                        min.saturating_sub(1)
                    );
                    Ok(format!(
                        r"\[{ws}{required_part}({ws},{ws}{item_regex})*{ws}\]"
                    ))
                }
            }
            (0, Some(max)) => {
                if max == 1 {
                    Ok(format!(r"\[{ws}({item_regex})?{ws}\]"))
                } else {
                    let max_part = max.saturating_sub(1);
                    Ok(format!(
                        r"\[{ws}({item_regex}({ws},{ws}{item_regex}){{0,{max_part}}})?{ws}\]"
                    ))
                }
            }
            (min, Some(max)) => {
                if min == max {
                    if min == 0 {
                        Ok(format!(r"\[{ws}\]"))
                    } else if min == 1 {
                        Ok(format!(r"\[{ws}{item_regex}{ws}\]"))
                    } else {
                        let count = min.saturating_sub(1);
                        Ok(format!(
                            r"\[{ws}{item_regex}({ws},{ws}{item_regex}){{{count}}}{ws}\]"
                        ))
                    }
                } else {
                    let min_part = if min > 0 {
                        format!(
                            "{item_regex}({ws},{ws}{item_regex}){{{}}}",
                            min.saturating_sub(1)
                        )
                    } else {
                        String::new()
                    };
                    let max_additional = max.saturating_sub(min);

                    if min_part.is_empty() {
                        Ok(format!(
                            r"\[{ws}({item_regex}({ws},{ws}{item_regex}){{0,{max_additional}}})?{ws}\]"
                        ))
                    } else {
                        Ok(format!(
                            r"\[{ws}{min_part}({ws},{ws}{item_regex}){{0,{max_additional}}}{ws}\]"
                        ))
                    }
                }
            }
        }
    }

    /// Generate regex for object type (delegates to properties if available)
    fn generate_object_regex(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        if obj.contains_key("properties") {
            self.parse_properties(obj)
        } else {
            let ws = self.whitespace_pattern;
            Ok(format!(r"\{{{ws}.*{ws}\}}"))
        }
    }

    /// Parse enum constraints
    fn parse_enum(&self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let enum_values = obj
            .get("enum")
            .and_then(Value::as_array)
            .context("Enum field not found or invalid")?;

        let mut patterns = Vec::with_capacity(enum_values.len());
        for value in enum_values {
            let pattern = match value {
                Value::String(s) => {
                    let escaped = escape(s);
                    format!(r#""{escaped}""#)
                }
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                _ => continue,
            };
            patterns.push(pattern);
        }

        if patterns.is_empty() {
            return Ok(r".*".to_string());
        }

        Ok(patterns.join("|"))
    }

    /// Parse const constraints
    fn parse_const(&self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let const_value = obj.get("const").context("Const field not found")?;
        match const_value {
            Value::String(s) => {
                let escaped = escape(s);
                Ok(format!(r#""{escaped}""#))
            }
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Ok("null".to_string()),
            _ => Ok(escape(&const_value.to_string())),
        }
    }

    /// Parse allOf constraints (intersection)
    fn parse_all_of(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let all_of = obj
            .get("allOf")
            .and_then(Value::as_array)
            .context("AllOf field not found or invalid")?;

        // For allOf, we need intersection logic
        // Simplified to merge properties from all schemas
        let mut merged_properties = HashMap::new();
        let mut merged_required = FxHashSet::default();
        let mut type_constraints = Vec::new();

        for schema in all_of {
            if let Value::Object(schema_obj) = schema {
                // Collect properties
                if let Some(Value::Object(props)) = schema_obj.get("properties") {
                    for (key, value) in props {
                        merged_properties.insert(key.clone(), value.clone());
                    }
                }

                // Collect required fields
                if let Some(Value::Array(required)) = schema_obj.get("required") {
                    for req in required {
                        if let Value::String(req_str) = req {
                            merged_required.insert(req_str.clone());
                        }
                    }
                }

                // Collect type constraints
                if let Some(type_val) = schema_obj.get("type") {
                    type_constraints.push(type_val);
                }
            }
        }

        // Build merged schema
        let mut merged_schema = json!({});
        if !merged_properties.is_empty() {
            merged_schema["properties"] = json!(merged_properties);
        }
        if !merged_required.is_empty() {
            let required_vec: Vec<&String> = merged_required.iter().collect();
            merged_schema["required"] = json!(required_vec);
        }
        if let Some(first_type) = type_constraints.first() {
            merged_schema["type"] = (*first_type).clone();
        }

        self.to_regex(&merged_schema)
    }

    /// Parse anyOf constraints (union)
    fn parse_any_of(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let any_of = obj
            .get("anyOf")
            .and_then(Value::as_array)
            .context("AnyOf field not found or invalid")?;

        let mut patterns = Vec::with_capacity(any_of.len());
        for schema in any_of {
            let pattern = self.to_regex(schema)?;
            patterns.push(format!("({pattern})"));
        }

        if patterns.is_empty() {
            return Ok(r".*".to_string());
        }

        Ok(patterns.join("|"))
    }

    /// Parse oneOf constraints (exclusive union)
    fn parse_one_of(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        // OneOf is similar to anyOf for regex generation purposes
        self.parse_any_of(obj)
    }

    /// Parse prefixItems for tuple validation
    fn parse_prefix_items(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let prefix_items = obj
            .get("prefixItems")
            .and_then(Value::as_array)
            .context("PrefixItems field not found or invalid")?;

        let ws = self.whitespace_pattern;

        if prefix_items.is_empty() {
            return Ok(format!(r"\[{ws}\]"));
        }

        let mut patterns = Vec::with_capacity(prefix_items.len());
        for schema in prefix_items {
            let pattern = self.to_regex(schema)?;
            patterns.push(pattern);
        }

        let items_pattern = patterns.join(&format!("{ws},{ws}"));

        // Handle additional items
        let additional_items = obj.get("items").unwrap_or(&json!(true));
        match additional_items {
            Value::Bool(false) => {
                // No additional items allowed
                Ok(format!(r"\[{ws}{items_pattern}{ws}\]"))
            }
            _ => {
                // Additional items allowed
                let additional_regex = if let Value::Bool(true) = additional_items {
                    r"[^,\[\]]*".to_string()
                } else {
                    self.to_regex(additional_items)?
                };
                Ok(format!(
                    r"\[{ws}{items_pattern}({ws},{ws}{})*{ws}\]",
                    additional_regex
                ))
            }
        }
    }

    /// Parse arrays with items schema
    fn parse_array_with_items(
        &mut self,
        obj: &serde_json::Map<String, Value>,
    ) -> AnyResult<String> {
        // Delegate to generate_array_regex which handles items
        self.generate_array_regex(obj)
    }

    /// Parse $ref constraints (JSON pointer resolution)
    fn parse_ref(&mut self, obj: &serde_json::Map<String, Value>) -> AnyResult<String> {
        let ref_path = obj
            .get("$ref")
            .and_then(Value::as_str)
            .context("$ref field not found or invalid")?;

        if ref_path.starts_with("#/") {
            // Try to resolve within root document using JSON pointer
            let path_parts: Vec<&str> = ref_path
                .strip_prefix("#/")
                .unwrap_or(ref_path)
                .split('/')
                .collect();
            let mut current = self.root;
            let empty_object = json!({});

            for part in path_parts {
                // Handle escaped JSON pointer characters
                let unescaped_part = part.replace("~1", "/").replace("~0", "~");
                current = current.get(&unescaped_part).unwrap_or(&empty_object);
            }

            if current != &empty_object {
                self.to_regex(current)
            } else {
                Ok(r".*".to_string())
            }
        } else {
            // External references not supported
            Ok(r".*".to_string())
        }
    }
}

/// Main entry point for converting JSON schema to regex
pub fn regex_from_value(
    json: &Value,
    whitespace_pattern: Option<&str>,
    max_recursion_depth: Option<usize>,
) -> AnyResult<String> {
    let mut parser = SchemaParser::new(json);

    if let Some(pattern) = whitespace_pattern {
        parser = parser.with_whitespace_pattern(pattern);
    }

    if let Some(depth) = max_recursion_depth {
        parser = parser.with_max_recursion_depth(depth);
    }

    parser.to_regex(json)
}

/// Convert schemars schema to regex pattern
pub fn regex_from_schema<T>() -> AnyResult<String>
where
    T: schemars::JsonSchema + serde::Serialize,
{
    let schema = schemars::schema_for!(T);
    let schema_value = serde_json::to_value(schema)?;
    regex_from_value(&schema_value, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, schemars::JsonSchema)]
    struct TestStruct {
        name: String,
        age: i32,
        active: bool,
    }

    #[test]
    fn test_simple_object_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        });

        let regex = match regex_from_value(&schema, None, None) {
            Ok(r) => r,
            Err(e) => panic!("Should generate regex: {}", e),
        };
        assert!(regex.contains("name"));
        assert!(regex.contains("age"));
    }

    #[test]
    fn test_schema_from_serde_type() {
        let regex = match regex_from_schema::<TestStruct>() {
            Ok(r) => r,
            Err(e) => panic!("Should generate regex from serde type: {}", e),
        };
        assert!(regex.contains("name"));
        assert!(regex.contains("age"));
        assert!(regex.contains("active"));
    }

    #[test]
    fn test_enum_schema() {
        let schema = json!({
            "enum": ["red", "green", "blue"]
        });

        let regex = match regex_from_value(&schema, None, None) {
            Ok(r) => r,
            Err(e) => panic!("Should generate regex: {}", e),
        };
        assert!(regex.contains("red"));
        assert!(regex.contains("green"));
        assert!(regex.contains("blue"));
    }

    #[test]
    fn test_array_schema() {
        let schema = json!({
            "type": "array",
            "items": {"type": "string"},
            "minItems": 1,
            "maxItems": 3
        });

        let regex = match regex_from_value(&schema, None, None) {
            Ok(r) => r,
            Err(e) => panic!("Should generate regex: {}", e),
        };
        assert!(regex.starts_with(r"\["));
        assert!(regex.ends_with(r"\]"));
    }
}
