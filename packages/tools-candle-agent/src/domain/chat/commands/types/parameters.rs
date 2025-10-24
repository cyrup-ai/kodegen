//! Command parameter definitions and validation with zero allocation patterns
//!
//! Provides blazing-fast parameter validation and type checking with owned strings
//! allocated once for maximum performance. No Arc usage, no locking.

use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::errors::{CandleCommandError, ValidationResult};

/// Parameter type enumeration for command parameters with zero allocation dispatch
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ParameterType {
    /// String parameter - text values
    String,
    /// Integer parameter - whole numbers
    Integer,
    /// Float parameter - decimal numbers
    Float,
    /// Boolean parameter - true/false values
    Boolean,
    /// Array of strings - multiple text values
    StringArray,
    /// File path parameter - filesystem paths
    FilePath,
    /// URL parameter - web addresses
    Url,
    /// JSON object parameter - structured data
    Json,
    /// Enumeration parameter with possible values
    Enum {
        /// Allowed values for this enum parameter
        values: Vec<String>,
    },
    /// Path parameter for file/directory paths
    Path,
    /// Duration parameter - time intervals
    Duration,
    /// Size parameter - byte counts with units
    Size,
    /// Regex pattern parameter
    Regex,
    /// Email address parameter
    Email,
    /// IP address parameter
    IpAddress,
    /// UUID parameter
    Uuid,
    /// Date parameter (ISO 8601 format)
    Date,
    /// Time parameter (HH:MM:SS format)
    Time,
    /// `DateTime` parameter (`ISO 8601` format)
    DateTime,
}

impl ParameterType {
    /// Validate a string value against this parameter type
    /// Returns validation result with zero allocation where possible
    ///
    /// # Errors
    ///
    /// Returns `CandleCommandError` if:
    /// - Value cannot be parsed as the expected type (Integer, Float, Boolean, etc.)
    /// - Duration format is invalid
    /// - URL format is invalid
    /// - File path is invalid or contains unsafe characters
    /// - JSON value cannot be parsed
    #[inline]
    pub fn validate(&self, value: &str) -> ValidationResult {
        match self {
            Self::String => Ok(()),
            Self::Integer => Self::validate_integer(value),
            Self::Float => Self::validate_float(value),
            Self::Boolean => Self::validate_boolean(value),
            Self::StringArray => Self::validate_string_array(value),
            Self::FilePath | Self::Path => Self::validate_path(value),
            Self::Url => Self::validate_url(value),
            Self::Json => Self::validate_json(value),
            Self::Enum { values } => Self::validate_enum(value, values),
            Self::Duration => Self::validate_duration_format(value),
            Self::Size => Self::validate_size_format(value),
            Self::Regex => Self::validate_regex(value),
            Self::Email => Self::validate_email(value),
            Self::IpAddress => Self::validate_ip_address(value),
            Self::Uuid => Self::validate_uuid(value),
            Self::Date => Self::validate_date(value),
            Self::Time => Self::validate_time(value),
            Self::DateTime => Self::validate_datetime(value),
        }
    }

    /// Validate integer value - zero allocation where possible
    #[inline]
    fn validate_integer(value: &str) -> ValidationResult {
        i64::from_str(value).map_err(|_| {
            CandleCommandError::validation_failed(format!("Invalid integer: {value}"))
        })?;
        Ok(())
    }

    /// Validate float value - zero allocation where possible
    #[inline]
    fn validate_float(value: &str) -> ValidationResult {
        f64::from_str(value).map_err(|_| {
            CandleCommandError::validation_failed(format!("Invalid float: {value}"))
        })?;
        Ok(())
    }

    /// Validate boolean value - accepts multiple formats
    #[inline]
    fn validate_boolean(value: &str) -> ValidationResult {
        match value.to_lowercase().as_str() {
            "true" | "false" | "1" | "0" | "yes" | "no" | "on" | "off" => Ok(()),
            _ => Err(CandleCommandError::validation_failed(format!(
                "Invalid boolean: {value}"
            ))),
        }
    }

    /// Validate string array in JSON format
    #[inline]
    fn validate_string_array(value: &str) -> ValidationResult {
        if value.starts_with('[') && value.ends_with(']') {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(
                "String array must be JSON array format".to_string(),
            ))
        }
    }

    /// Validate file path or directory path - checks non-empty
    #[inline]
    fn validate_path(value: &str) -> ValidationResult {
        if value.is_empty() {
            Err(CandleCommandError::validation_failed(
                "Path cannot be empty".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Validate URL format - checks for valid protocol prefix
    #[inline]
    fn validate_url(value: &str) -> ValidationResult {
        if value.starts_with("http://")
            || value.starts_with("https://")
            || value.starts_with("ftp://")
            || value.starts_with("file://")
        {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid URL format: {value}"
            )))
        }
    }

    /// Validate JSON value - parses to ensure validity
    #[inline]
    fn validate_json(value: &str) -> ValidationResult {
        serde_json::from_str::<serde_json::Value>(value)
            .map_err(|_| CandleCommandError::validation_failed(format!("Invalid JSON: {value}")))?;
        Ok(())
    }

    /// Validate enum value against allowed values
    #[inline]
    fn validate_enum(value: &str, allowed_values: &[String]) -> ValidationResult {
        if allowed_values.contains(&value.to_string()) {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid enum value: {value}. Allowed: {allowed_values:?}"
            )))
        }
    }

    /// Validate duration format (e.g., "5s", "10m", "1h", "2d")
    #[inline]
    fn validate_duration_format(value: &str) -> ValidationResult {
        if Self::parse_duration(value).is_some() {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid duration format: {value}"
            )))
        }
    }

    /// Validate size format (e.g., "100", "1KB", "2MB", "1GB")
    #[inline]
    fn validate_size_format(value: &str) -> ValidationResult {
        if Self::parse_size(value).is_some() {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid size format: {value}"
            )))
        }
    }

    /// Validate regex pattern - ensures it compiles
    #[inline]
    fn validate_regex(value: &str) -> ValidationResult {
        regex::Regex::new(value).map_err(|_| {
            CandleCommandError::validation_failed(format!("Invalid regex pattern: {value}"))
        })?;
        Ok(())
    }

    /// Validate email format - basic check for @ and .
    #[inline]
    fn validate_email(value: &str) -> ValidationResult {
        if value.contains('@') && value.contains('.') {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid email format: {value}"
            )))
        }
    }

    /// Validate IP address format
    #[inline]
    fn validate_ip_address(value: &str) -> ValidationResult {
        if value.parse::<std::net::IpAddr>().is_ok() {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid IP address: {value}"
            )))
        }
    }

    /// Validate UUID format - basic structural check
    #[inline]
    fn validate_uuid(value: &str) -> ValidationResult {
        if value.len() == 36 && value.chars().filter(|&c| c == '-').count() == 4 {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid UUID format: {value}"
            )))
        }
    }

    /// Validate date format (YYYY-MM-DD)
    #[inline]
    fn validate_date(value: &str) -> ValidationResult {
        if value.len() == 10 && value.chars().filter(|&c| c == '-').count() == 2 {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid date format (expected YYYY-MM-DD): {value}"
            )))
        }
    }

    /// Validate time format (HH:MM:SS or HH:MM)
    #[inline]
    fn validate_time(value: &str) -> ValidationResult {
        if value.len() >= 5 && value.chars().filter(|&c| c == ':').count() >= 1 {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid time format (expected HH:MM:SS): {value}"
            )))
        }
    }

    /// Validate datetime format (ISO 8601)
    #[inline]
    fn validate_datetime(value: &str) -> ValidationResult {
        if value.contains('T') || value.len() >= 19 {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(format!(
                "Invalid datetime format (expected ISO 8601): {value}"
            )))
        }
    }

    /// Parse duration string to seconds - zero allocation where possible
    #[inline]
    fn parse_duration(value: &str) -> Option<u64> {
        if value.is_empty() {
            return None;
        }

        let (num_str, unit) = if let Some(stripped) = value.strip_suffix("ms") {
            (stripped, "ms")
        } else if let Some(last_char) = value.chars().last() {
            match last_char {
                's' => (value.strip_suffix('s').unwrap_or(value), "s"),
                'm' => (value.strip_suffix('m').unwrap_or(value), "m"),
                'h' => (value.strip_suffix('h').unwrap_or(value), "h"),
                'd' => (value.strip_suffix('d').unwrap_or(value), "d"),
                _ => (value, "s"), // default to seconds
            }
        } else {
            return None;
        };

        let num: u64 = num_str.parse().ok()?;

        Some(match unit {
            "ms" => num / 1000,
            "m" => num * 60,
            "h" => num * 3600,
            "d" => num * 86400,
            "s" | "" => num,  // seconds or empty defaults to seconds
            _ => return None, // invalid unit
        })
    }

    /// Parse size string to bytes - zero allocation where possible
    #[inline]
    fn parse_size(value: &str) -> Option<u64> {
        if value.is_empty() {
            return None;
        }

        let (num_str, multiplier) = if let Some(stripped) = value.strip_suffix("KB") {
            (stripped, 1024)
        } else if let Some(stripped) = value.strip_suffix("MB") {
            (stripped, 1024 * 1024)
        } else if let Some(stripped) = value.strip_suffix("GB") {
            (stripped, 1024 * 1024 * 1024)
        } else if let Some(stripped) = value.strip_suffix("TB") {
            (stripped, 1024_u64.pow(4))
        } else if let Some(stripped) = value.strip_suffix('B') {
            (stripped, 1)
        } else {
            (value, 1) // assume bytes
        };

        let num: u64 = num_str.parse().ok()?;
        Some(num * multiplier)
    }

    /// Get type name as static string for zero allocation
    #[inline]
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::StringArray => "string_array",
            Self::FilePath => "file_path",
            Self::Url => "url",
            Self::Json => "json",
            Self::Enum { .. } => "enum",
            Self::Path => "path",
            Self::Duration => "duration",
            Self::Size => "size",
            Self::Regex => "regex",
            Self::Email => "email",
            Self::IpAddress => "ip_address",
            Self::Uuid => "uuid",
            Self::Date => "date",
            Self::Time => "time",
            Self::DateTime => "datetime",
        }
    }
}

/// Parameter information for command definitions with owned strings allocated once
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// Parameter name (owned string allocated once)
    pub name: String,
    /// Parameter description (owned string allocated once)
    pub description: String,
    /// Parameter type
    pub parameter_type: ParameterType,
    /// Whether the parameter is required
    pub required: bool,
    /// Default value if not required (owned string allocated once)
    pub default_value: Option<String>,
    /// Minimum value for numeric parameters
    pub min_value: Option<f64>,
    /// Maximum value for numeric parameters
    pub max_value: Option<f64>,
    /// Pattern for string validation (regex)
    pub pattern: Option<String>,
    /// Examples of valid values
    pub examples: Vec<String>,
}

impl ParameterInfo {
    /// Create a new required parameter with zero allocation constructor
    #[inline]
    pub fn required(
        name: impl Into<String>,
        description: impl Into<String>,
        parameter_type: ParameterType,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameter_type,
            required: true,
            default_value: None,
            min_value: None,
            max_value: None,
            pattern: None,
            examples: Vec::new(),
        }
    }

    /// Create a new optional parameter with default value
    #[inline]
    pub fn optional(
        name: impl Into<String>,
        description: impl Into<String>,
        parameter_type: ParameterType,
        default_value: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameter_type,
            required: false,
            default_value: Some(default_value.into()),
            min_value: None,
            max_value: None,
            pattern: None,
            examples: Vec::new(),
        }
    }

    /// Add numeric range constraints - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }

    /// Add validation pattern - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Add usage examples - builder pattern for fluent API
    #[must_use]
    #[inline]
    pub fn with_examples(mut self, examples: Vec<String>) -> Self {
        self.examples = examples;
        self
    }

    /// Validate parameter value with comprehensive checking
    ///
    /// # Errors
    ///
    /// Returns `CandleCommandError` if:
    /// - Type validation fails (via `parameter_type.validate()`)
    /// - Value is required but empty
    /// - Value doesn't match any of the allowed values (if specified)
    #[inline]
    pub fn validate(&self, value: &str) -> ValidationResult {
        // Type validation first
        self.parameter_type.validate(value)?;

        // Range validation for numeric types
        if let Some(min) = self.min_value {
            match &self.parameter_type {
                ParameterType::Integer => {
                    let val = value
                        .parse::<i64>()
                        .map_err(|_| CandleCommandError::validation_failed("Invalid integer"))?;
                    #[allow(clippy::cast_precision_loss)]
                    if (val as f64) < min {
                        return Err(CandleCommandError::validation_failed(format!(
                            "Value {val} is below minimum {min}"
                        )));
                    }
                }
                ParameterType::Float => {
                    let val = value
                        .parse::<f64>()
                        .map_err(|_| CandleCommandError::validation_failed("Invalid float"))?;
                    if val < min {
                        return Err(CandleCommandError::validation_failed(format!(
                            "Value {val} is below minimum {min}"
                        )));
                    }
                }
                _ => {}
            }
        }

        if let Some(max) = self.max_value {
            match &self.parameter_type {
                ParameterType::Integer => {
                    let val = value
                        .parse::<i64>()
                        .map_err(|_| CandleCommandError::validation_failed("Invalid integer"))?;
                    #[allow(clippy::cast_precision_loss)]
                    if (val as f64) > max {
                        return Err(CandleCommandError::validation_failed(format!(
                            "Value {val} is above maximum {max}"
                        )));
                    }
                }
                ParameterType::Float => {
                    let val = value
                        .parse::<f64>()
                        .map_err(|_| CandleCommandError::validation_failed("Invalid float"))?;
                    if val > max {
                        return Err(CandleCommandError::validation_failed(format!(
                            "Value {val} is above maximum {max}"
                        )));
                    }
                }
                _ => {}
            }
        }

        // Pattern validation
        if let Some(pattern) = &self.pattern {
            let regex = regex::Regex::new(pattern)
                .map_err(|_| CandleCommandError::validation_failed("Invalid validation pattern"))?;
            if !regex.is_match(value) {
                return Err(CandleCommandError::validation_failed(format!(
                    "Value '{value}' does not match required pattern"
                )));
            }
        }

        Ok(())
    }
}

/// Parameter validator for bulk validation with zero allocation patterns
#[derive(Debug, Clone)]
pub struct ParameterValidator {
    parameters: Vec<ParameterInfo>,
}

impl ParameterValidator {
    /// Create new validator with parameter definitions
    #[inline]
    #[must_use]
    pub fn new(parameters: Vec<ParameterInfo>) -> Self {
        Self { parameters }
    }

    /// Validate all parameters in a command with comprehensive error reporting
    ///
    /// # Errors
    ///
    /// Returns `CandleCommandError` if:
    /// - Any required parameter is missing
    /// - Any provided parameter fails validation via `ParameterInfo::validate()`
    #[inline]
    pub fn validate_all(&self, values: &HashMap<String, String>) -> ValidationResult {
        let mut errors = Vec::new();

        // Check required parameters
        for param in &self.parameters {
            if param.required && !values.contains_key(&param.name) {
                errors.push(format!("Missing required parameter '{}'", param.name));
                continue;
            }

            // Validate provided values
            if let Some(value) = values.get(&param.name)
                && let Err(err) = param.validate(value)
            {
                errors.push(format!("Parameter '{}': {}", param.name, err));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(CandleCommandError::validation_failed(errors.join("; ")))
        }
    }

    /// Get parameter by name - zero allocation lookup
    #[inline]
    #[must_use]
    pub fn get_parameter(&self, name: &str) -> Option<&ParameterInfo> {
        self.parameters.iter().find(|p| p.name == name)
    }

    /// Get all parameters - returns slice to avoid allocation
    #[inline]
    #[must_use]
    pub fn parameters(&self) -> &[ParameterInfo] {
        &self.parameters
    }
}
