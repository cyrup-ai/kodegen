//! Template parser implementation
//!
//! Provides high-performance parsing for template syntax with
//! zero-allocation patterns where possible.

use std::collections::HashMap;

use super::core::{TemplateAst, TemplateError, TemplateResult, TemplateVariable, VariableType};

// Declare internal modules
mod blocks;
mod config;
mod expressions;
mod functions;
mod utils;

// Re-export public API
pub use config::ParserConfig;

/// Template parser implementation
#[derive(Debug)]
pub struct TemplateParser {
    config: ParserConfig,
}

impl TemplateParser {
    /// Create a new template parser with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }

    /// Create a new template parser with custom configuration
    #[must_use]
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }

    /// Parse template content into AST
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if:
    /// - Maximum parsing depth is exceeded
    /// - Template syntax is invalid
    /// - Variable or expression parsing fails
    pub fn parse(&self, content: &str) -> TemplateResult<TemplateAst> {
        self.parse_with_depth(content, 0)
    }

    /// Parse template content with depth tracking
    fn parse_with_depth(&self, content: &str, depth: usize) -> TemplateResult<TemplateAst> {
        if depth > self.config.max_depth {
            return Err(TemplateError::ParseError {
                message: "Maximum parsing depth exceeded".to_string(),
            });
        }

        let chars: Vec<char> = content.chars().collect();
        let nodes = self.parse_template_nodes(content, depth, &chars)?;

        match nodes.len() {
            0 => Ok(TemplateAst::Text(String::new())),
            1 => Ok(nodes
                .into_iter()
                .next()
                .unwrap_or(TemplateAst::Text(String::new()))),
            _ => Ok(TemplateAst::Block(nodes.into())),
        }
    }

    /// Parse template nodes from character stream
    fn parse_template_nodes(
        &self,
        content: &str,
        depth: usize,
        chars: &[char],
    ) -> TemplateResult<Vec<TemplateAst>> {
        let mut nodes = Vec::new();
        let mut current_text = String::new();
        let mut i = 0;

        while i < chars.len() {
            // Check for {% block start %}
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '%' {
                if !current_text.is_empty() {
                    nodes.push(TemplateAst::Text(current_text.clone()));
                    current_text.clear();
                }

                let parse_result = self.parse_block_tag(content, i, depth, chars)?;
                match parse_result {
                    utils::BlockTagResult::Parsed(ast, new_i) => {
                        nodes.push(ast);
                        i = new_i;
                    }
                    utils::BlockTagResult::EndTag => break,
                    utils::BlockTagResult::Skip(new_i) => {
                        i = new_i;
                    }
                }
            }
            // Check for {{ expression }}
            else if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                if !current_text.is_empty() {
                    nodes.push(TemplateAst::Text(current_text.clone()));
                    current_text.clear();
                }

                let (ast_node, new_i) = self.parse_expression_tag(chars, i, depth)?;
                nodes.push(ast_node);
                i = new_i;
            } else {
                current_text.push(chars[i]);
                i += 1;
            }
        }

        if !current_text.is_empty() {
            nodes.push(TemplateAst::Text(current_text));
        }

        Ok(nodes)
    }

    /// Extract variables from template content
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if variable extraction fails
    pub fn extract_variables(&self, content: &str) -> TemplateResult<Vec<TemplateVariable>> {
        let mut variables = HashMap::new();
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();

        while i < chars.len() {
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                i += 2;

                // Find closing }}
                let mut expr_end = i;
                let mut brace_count = 0;

                while expr_end < chars.len() {
                    if expr_end + 1 < chars.len()
                        && chars[expr_end] == '}'
                        && chars[expr_end + 1] == '}'
                    {
                        if brace_count == 0 {
                            break;
                        }
                        brace_count -= 1;
                        expr_end += 1;
                    } else if expr_end + 1 < chars.len()
                        && chars[expr_end] == '{'
                        && chars[expr_end + 1] == '{'
                    {
                        brace_count += 1;
                        expr_end += 1;
                    }
                    expr_end += 1;
                }

                let var_content: String = chars[i..expr_end].iter().collect();
                let var_name = var_content.trim();

                // Skip control structures
                if var_name.starts_with("if ")
                    || var_name.starts_with("for ")
                    || var_name.starts_with("end")
                {
                    i = expr_end + 2;
                    continue;
                }

                // Extract simple variable name (before any operators or functions)
                let clean_name = var_name
                    .split_whitespace()
                    .next()
                    .unwrap_or(var_name)
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .trim();

                if !clean_name.is_empty()
                    && clean_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    variables.insert(
                        clean_name.to_string(),
                        TemplateVariable {
                            name: clean_name.to_string(),
                            description: String::new(),
                            var_type: VariableType::String,
                            default_value: None,
                            required: false,
                            validation_pattern: None,
                            valid_values: None,
                            min_value: None,
                            max_value: None,
                        },
                    );
                }

                i = expr_end + 2;
            } else {
                i += 1;
            }
        }

        Ok(variables.into_values().collect())
    }
}

impl Default for TemplateParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parser statistics for performance monitoring
#[derive(Debug, Clone, PartialEq)]
pub struct ParserStats {
    /// Total number of templates parsed
    pub parsed_count: usize,
    /// Total time spent parsing in microseconds
    pub total_parse_time_us: usize,
    /// Number of parse errors encountered
    pub error_count: usize,
    /// Peak memory usage during parsing in bytes
    pub peak_memory_bytes: usize,
}

/// Parse error type alias for convenience
pub type ParseError = TemplateError;

/// Parse result type alias for convenience
pub type ParseResult<T> = Result<T, ParseError>;

/// Quick parse function for convenience
///
/// # Errors
///
/// Returns `TemplateError` if parsing fails (see `TemplateParser::parse`)
pub fn parse_template(content: &str) -> TemplateResult<TemplateAst> {
    let parser = TemplateParser::new();
    parser.parse(content)
}

/// Extract variables from template content
///
/// # Errors
///
/// Returns `TemplateError` if variable extraction fails
pub fn extract_variables(content: &str) -> TemplateResult<Vec<TemplateVariable>> {
    let parser = TemplateParser::new();
    parser.extract_variables(content)
}

/// Validate template syntax
///
/// # Errors
///
/// Returns `TemplateError` if template syntax is invalid
pub fn validate_template(content: &str) -> TemplateResult<()> {
    let parser = TemplateParser::new();
    parser.parse(content)?;
    Ok(())
}
