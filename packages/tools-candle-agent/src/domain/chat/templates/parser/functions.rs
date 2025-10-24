//! Function call parsing

use super::super::core::{TemplateAst, TemplateError, TemplateResult};
use super::TemplateParser;

impl TemplateParser {
    /// Parse a function call expression
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if:
    /// - Function calls are not allowed in config
    /// - Maximum parsing depth is exceeded
    /// - Function call syntax is invalid
    pub(crate) fn parse_function_call(
        &self,
        content: &str,
        depth: usize,
    ) -> TemplateResult<TemplateAst> {
        if !self.config.allow_functions {
            return Err(TemplateError::ParseError {
                message: "Function calls not allowed".to_string(),
            });
        }

        if depth > self.config.max_depth {
            return Err(TemplateError::ParseError {
                message: "Maximum parsing depth exceeded".to_string(),
            });
        }

        let paren_pos = content.find('(').ok_or_else(|| TemplateError::ParseError {
            message: "Invalid function call syntax".to_string(),
        })?;
        let func_name = content[..paren_pos].trim();

        // Extract arguments
        let close_paren = content
            .rfind(')')
            .ok_or_else(|| TemplateError::ParseError {
                message: "Unclosed function call".to_string(),
            })?;

        let args_str = &content[paren_pos + 1..close_paren];

        let args = if args_str.trim().is_empty() {
            Vec::new()
        } else {
            self.parse_function_args(args_str, depth)?
        };

        Ok(TemplateAst::Function {
            name: func_name.to_string(),
            args: args.into(),
        })
    }

    /// Parse function arguments with parenthesis and quote awareness
    ///
    /// Handles nested function calls by tracking parenthesis depth and quote context.
    /// Only splits on commas when at depth 0 and not inside a string literal.
    ///
    /// # Arguments
    ///
    /// * `args_str` - The argument string to parse (content between parentheses)
    /// * `depth` - Current parsing depth for recursion tracking
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if argument parsing fails
    pub(crate) fn parse_function_args(
        &self,
        args_str: &str,
        depth: usize,
    ) -> TemplateResult<Vec<TemplateAst>> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut paren_depth = 0;
        let mut string_delimiter: Option<char> = None; // Track which quote opened the string

        for ch in args_str.chars() {
            match ch {
                '"' | '\'' => {
                    match string_delimiter {
                        None => string_delimiter = Some(ch), // Open string
                        Some(delim) if delim == ch => string_delimiter = None, // Close if matching
                        _ => {} // Different quote inside string, ignore
                    }
                }
                '(' if string_delimiter.is_none() => paren_depth += 1,
                ')' if string_delimiter.is_none() => paren_depth -= 1,
                ',' if string_delimiter.is_none() && paren_depth == 0 => {
                    args.push(self.parse_expression(current_arg.trim(), depth + 1)?);
                    current_arg.clear();
                    continue;
                }
                _ => {}
            }
            current_arg.push(ch);
        }

        if !current_arg.trim().is_empty() {
            args.push(self.parse_expression(current_arg.trim(), depth + 1)?);
        }

        Ok(args)
    }
}
