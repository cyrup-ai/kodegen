//! Expression and variable parsing

use std::sync::Arc;

use super::super::core::{TemplateAst, TemplateError, TemplateResult};
use super::TemplateParser;
use super::utils::{extract_operator, find_operator};

impl TemplateParser {
    /// Parse an expression (could be variable, binary operation, comparison, or function call)
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if expression parsing fails
    pub(crate) fn parse_expression(
        &self,
        content: &str,
        depth: usize,
    ) -> TemplateResult<TemplateAst> {
        if depth > self.config.max_depth {
            return Err(TemplateError::ParseError {
                message: "Maximum parsing depth exceeded".to_string(),
            });
        }

        let content = content.trim();

        // Check for string literals
        if (content.starts_with('"') && content.ends_with('"'))
            || (content.starts_with('\'') && content.ends_with('\''))
        {
            let literal = &content[1..content.len() - 1];
            return Ok(TemplateAst::Text(literal.to_string()));
        }

        // Check for numeric literals - store as text
        if content.parse::<f64>().is_ok() {
            return Ok(TemplateAst::Text(content.to_string()));
        }

        // Check for boolean literals - store as text
        if content == "true" || content == "false" {
            return Ok(TemplateAst::Text(content.to_string()));
        }

        // Check for logical operators (lowest precedence)
        if let Some(pos) = find_operator(content, &["||", "&&"]) {
            return self.parse_binary_operation(content, pos, &["||", "&&"], depth);
        }

        // Check for comparison operators
        if let Some(pos) = find_operator(content, &["==", "!=", "<=", ">=", "<", ">"]) {
            return self.parse_comparison(content, pos, &["==", "!=", "<=", ">=", "<", ">"], depth);
        }

        // Check for arithmetic operators (higher precedence)
        if let Some(pos) = find_operator(content, &["+", "-"]) {
            return self.parse_binary_operation(content, pos, &["+", "-"], depth);
        }

        if let Some(pos) = find_operator(content, &["*", "/", "%"]) {
            return self.parse_binary_operation(content, pos, &["*", "/", "%"], depth);
        }

        // Check for function calls
        if content.contains('(') && content.contains(')') {
            return self.parse_function_call(content, depth);
        }

        // Otherwise, treat as variable or simple expression
        self.parse_variable_or_expression(content, depth)
    }

    /// Parse a variable or simple expression
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if variable parsing fails
    pub(crate) fn parse_variable_or_expression(
        &self,
        content: &str,
        depth: usize,
    ) -> TemplateResult<TemplateAst> {
        if depth > self.config.max_depth {
            return Err(TemplateError::ParseError {
                message: "Maximum parsing depth exceeded".to_string(),
            });
        }

        let content = content.trim();

        // Handle property access (e.g., user.name)
        if content.contains('.') {
            let parts: Vec<&str> = content.split('.').collect();
            if parts.len() >= 2 {
                let obj = parts[0].trim();
                let prop = parts[1..].join(".");
                return Ok(TemplateAst::Expression {
                    operator: ".".to_string(),
                    operands: Arc::new([
                        TemplateAst::Variable(obj.to_string()),
                        TemplateAst::Text(prop),
                    ]),
                });
            }
        }

        // Handle array access (e.g., items[0])
        if content.contains('[')
            && content.contains(']')
            && let Some(bracket_pos) = content.find('[')
        {
            let array_name = content[..bracket_pos].trim();
            if let Some(close_bracket) = content.rfind(']') {
                let index_str = content[bracket_pos + 1..close_bracket].trim();
                let index = self.parse_expression(index_str, depth + 1)?;
                return Ok(TemplateAst::Expression {
                    operator: "[]".to_string(),
                    operands: Arc::new([TemplateAst::Variable(array_name.to_string()), index]),
                });
            }
        }

        // Simple variable
        if content.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Ok(TemplateAst::Variable(content.to_string()));
        }

        // If we can't parse it, return as text
        Ok(TemplateAst::Text(content.to_string()))
    }

    /// Parse a binary operation (arithmetic or logical)
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if binary operation parsing fails
    pub(crate) fn parse_binary_operation(
        &self,
        content: &str,
        pos: usize,
        operators: &[&str],
        depth: usize,
    ) -> TemplateResult<TemplateAst> {
        if depth > self.config.max_depth {
            return Err(TemplateError::ParseError {
                message: "Maximum parsing depth exceeded".to_string(),
            });
        }

        let (left, op, right) = extract_operator(content, pos, operators)?;

        let left_ast = self.parse_expression(&left, depth + 1)?;
        let right_ast = self.parse_expression(&right, depth + 1)?;

        Ok(TemplateAst::Expression {
            operator: op,
            operands: Arc::new([left_ast, right_ast]),
        })
    }

    /// Parse a comparison operation
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if comparison parsing fails
    pub(crate) fn parse_comparison(
        &self,
        content: &str,
        pos: usize,
        operators: &[&str],
        depth: usize,
    ) -> TemplateResult<TemplateAst> {
        if depth > self.config.max_depth {
            return Err(TemplateError::ParseError {
                message: "Maximum parsing depth exceeded".to_string(),
            });
        }

        let (left, op, right) = extract_operator(content, pos, operators)?;

        let left_ast = self.parse_expression(&left, depth + 1)?;
        let right_ast = self.parse_expression(&right, depth + 1)?;

        Ok(TemplateAst::Expression {
            operator: op,
            operands: Arc::new([left_ast, right_ast]),
        })
    }
}
