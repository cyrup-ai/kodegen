//! Low-level parameter validation functions
//!
//! Provides reusable validation functions for common parameter types
//! including strings, integers, enums, paths, and structured data.

use super::errors::ValidationError;
use regex::Regex;
use std::collections::HashMap;

/// Validation configuration for `CommandValidator`
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub max_command_length: usize,
    pub max_parameter_count: usize,
    pub max_parameter_value_length: usize,
    pub allowed_extensions: Vec<String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_command_length: 1024,
            max_parameter_count: 50,
            max_parameter_value_length: 512,
            allowed_extensions: vec![
                "txt".to_string(),
                "md".to_string(),
                "json".to_string(),
                "csv".to_string(),
                "html".to_string(),
                "pdf".to_string(),
            ],
        }
    }
}

impl ValidationConfig {
    /// Validate string parameter with length and security checks
    ///
    /// # Errors
    /// Returns `ValidationError` if the string is empty (when `allow_empty` is false) or exceeds the maximum length.
    pub fn validate_string_parameter(
        &self,
        name: &str,
        value: &str,
        allow_empty: bool,
    ) -> Result<(), ValidationError> {
        // Check if empty when not allowed
        if !allow_empty && value.is_empty() {
            return Err(ValidationError::EmptyParameter {
                parameter: name.to_string(),
            });
        }

        // Check length
        if value.len() > self.max_parameter_value_length {
            return Err(ValidationError::ParameterTooLong {
                parameter: name.to_string(),
                max_length: self.max_parameter_value_length,
                actual_length: value.len(),
            });
        }

        Ok(())
    }

    /// Validate integer parameter within optional range
    ///
    /// # Errors
    /// Returns `ValidationError` if the value is outside the specified min/max range.
    pub fn validate_integer_parameter(
        name: &str,
        value: i64,
        min: Option<i64>,
        max: Option<i64>,
    ) -> Result<(), ValidationError> {
        if let Some(min_val) = min
            && value < min_val
        {
            return Err(ValidationError::ParameterOutOfRange {
                parameter: name.to_string(),
                value: value.to_string(),
                min: Some(min_val.to_string()),
                max: max.map(|m| m.to_string()),
            });
        }

        if let Some(max_val) = max
            && value > max_val
        {
            return Err(ValidationError::ParameterOutOfRange {
                parameter: name.to_string(),
                value: value.to_string(),
                min: min.map(|m| m.to_string()),
                max: Some(max_val.to_string()),
            });
        }

        Ok(())
    }

    /// Validate enum parameter against allowed values
    ///
    /// # Errors
    /// Returns `ValidationError` if the value is not in the allowed values list.
    pub fn validate_enum_parameter(
        name: &str,
        value: &str,
        allowed_values: &[&str],
    ) -> Result<(), ValidationError> {
        if !allowed_values.contains(&value) {
            return Err(ValidationError::InvalidEnumValue {
                parameter: name.to_string(),
                value: value.to_string(),
                allowed_values: allowed_values.iter().map(|s| (*s).to_string()).collect(),
            });
        }

        Ok(())
    }

    /// Validate path parameter
    ///
    /// # Errors
    /// Returns `ValidationError` if the path contains traversal attempts (..), has invalid file extension, or fails string validation.
    pub fn validate_path_parameter(&self, name: &str, path: &str) -> Result<(), ValidationError> {
        // Basic string validation
        self.validate_string_parameter(name, path, false)?;

        // Check for path traversal attempts
        if path.contains("..") {
            return Err(ValidationError::SecurityViolation {
                parameter: name.to_string(),
                detail: "Path traversal attempt detected".to_string(),
            });
        }

        // Validate file extension if present
        if let Some(ext_pos) = path.rfind('.') {
            let extension = &path[ext_pos + 1..];
            if !self
                .allowed_extensions
                .iter()
                .any(|ext| ext.as_ref() as &str == extension)
            {
                return Err(ValidationError::InvalidFileExtension {
                    parameter: name.to_string(),
                    extension: extension.to_string(),
                    allowed_extensions: self.allowed_extensions.clone(),
                });
            }
        }

        Ok(())
    }

    /// Validate configuration key
    ///
    /// # Errors
    /// Returns `ValidationError` if the key is not alphanumeric with dots, underscores, and hyphens.
    pub fn validate_config_key(&self, key: &str) -> Result<(), ValidationError> {
        self.validate_string_parameter("key", key, false)?;

        // Config keys should be alphanumeric with dots and underscores
        let config_key_regex =
            Regex::new(r"^[a-zA-Z0-9._-]+$").map_err(|e| ValidationError::SecurityViolation {
                parameter: "key".to_string(),
                detail: format!("Regex compilation error: {e}"),
            })?;
        if !config_key_regex.is_match(key) {
            return Err(ValidationError::InvalidParameterFormat {
                parameter: "key".to_string(),
                value: key.to_string(),
                expected_format: "alphanumeric with dots, underscores, and hyphens".to_string(),
            });
        }

        Ok(())
    }

    /// Validate configuration value
    ///
    /// # Errors
    /// Returns `ValidationError` if the value fails string validation.
    pub fn validate_config_value(&self, value: &str) -> Result<(), ValidationError> {
        self.validate_string_parameter("value", value, true)?;
        Ok(())
    }

    /// Validate name parameter (for templates, macros, etc.)
    ///
    /// # Errors
    /// Returns `ValidationError` if the name is not alphanumeric with underscores and hyphens.
    pub fn validate_name_parameter(
        &self,
        param_name: &str,
        name: &str,
    ) -> Result<(), ValidationError> {
        self.validate_string_parameter(param_name, name, false)?;

        // Names should be alphanumeric with underscores and hyphens
        let name_regex =
            Regex::new(r"^[a-zA-Z0-9_-]+$").map_err(|e| ValidationError::SecurityViolation {
                parameter: param_name.to_string(),
                detail: format!("Regex compilation error: {e}"),
            })?;
        if !name_regex.is_match(name) {
            return Err(ValidationError::InvalidParameterFormat {
                parameter: param_name.to_string(),
                value: name.to_string(),
                expected_format: "alphanumeric with underscores and hyphens".to_string(),
            });
        }

        Ok(())
    }

    /// Validate content parameter
    ///
    /// # Errors
    /// Returns `ValidationError` if the content exceeds maximum length or contains script injection attempts.
    pub fn validate_content_parameter(
        &self,
        name: &str,
        content: &str,
    ) -> Result<(), ValidationError> {
        // Allow longer content but still validate
        if content.len() > self.max_parameter_value_length * 4 {
            return Err(ValidationError::ParameterTooLong {
                parameter: name.to_string(),
                max_length: self.max_parameter_value_length * 4,
                actual_length: content.len(),
            });
        }

        // Check for script injection attempts
        let script_regex = Regex::new(r"<script[^>]*>.*?</script>").map_err(|e| {
            ValidationError::SecurityViolation {
                parameter: name.to_string(),
                detail: format!("Regex compilation error: {e}"),
            }
        })?;
        if script_regex.is_match(content) {
            return Err(ValidationError::SecurityViolation {
                parameter: name.to_string(),
                detail: "Script injection attempt detected".to_string(),
            });
        }

        Ok(())
    }

    /// Validate template/macro variables
    ///
    /// # Errors
    /// Returns `ValidationError` if there are too many variables or if any variable key/value fails validation.
    pub fn validate_variables(
        &self,
        variables: &HashMap<String, String>,
    ) -> Result<(), ValidationError> {
        if variables.len() > self.max_parameter_count {
            return Err(ValidationError::TooManyParameters {
                max_count: self.max_parameter_count,
                actual_count: variables.len(),
            });
        }

        for (key, value) in variables {
            self.validate_name_parameter("variable_key", key)?;
            self.validate_string_parameter("variable_value", value, true)?;
        }

        Ok(())
    }

    /// Validate tool arguments
    ///
    /// # Errors
    /// Returns `ValidationError` if there are too many arguments or if any argument key/value fails validation.
    pub fn validate_tool_args(
        &self,
        args: &HashMap<String, String>,
    ) -> Result<(), ValidationError> {
        if args.len() > self.max_parameter_count {
            return Err(ValidationError::TooManyParameters {
                max_count: self.max_parameter_count,
                actual_count: args.len(),
            });
        }

        for (key, value) in args {
            self.validate_string_parameter("arg_key", key, false)?;
            self.validate_string_parameter("arg_value", value, true)?;
        }

        Ok(())
    }

    /// Validate theme properties
    ///
    /// # Errors
    /// Returns `ValidationError` if there are too many properties or if any property key/value fails validation.
    pub fn validate_theme_properties(
        &self,
        properties: &HashMap<String, String>,
    ) -> Result<(), ValidationError> {
        if properties.len() > self.max_parameter_count {
            return Err(ValidationError::TooManyParameters {
                max_count: self.max_parameter_count,
                actual_count: properties.len(),
            });
        }

        for (key, value) in properties {
            self.validate_string_parameter("property_key", key, false)?;
            self.validate_string_parameter("property_value", value, true)?;
        }

        Ok(())
    }
}
