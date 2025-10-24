//! Block tag parsing (if/for/endif/endfor/elif/else)

use std::sync::Arc;

use super::super::core::{TemplateAst, TemplateError, TemplateResult};
use super::TemplateParser;
use super::utils::BlockTagResult;

impl TemplateParser {
    /// Parse a block tag like {% if %}, {% for %}, etc.
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if block tag parsing fails
    pub(crate) fn parse_block_tag(
        &self,
        content: &str,
        i: usize,
        depth: usize,
        chars: &[char],
    ) -> TemplateResult<BlockTagResult> {
        let block_start = i + 2;
        let mut block_end = block_start;
        while block_end + 1 < chars.len() {
            if chars[block_end] == '%' && chars[block_end + 1] == '}' {
                break;
            }
            block_end += 1;
        }

        if block_end + 1 >= chars.len() {
            return Err(TemplateError::ParseError {
                message: "Unclosed block tag".to_string(),
            });
        }

        let block_content: String = chars[block_start..block_end].iter().collect();
        let block_content = block_content.trim();

        if let Some(_tag_content) = block_content.strip_prefix("if ") {
            let (ast, new_i) = self.parse_conditional_block(content, i, depth)?;
            Ok(BlockTagResult::Parsed(ast, new_i))
        } else if let Some(_tag_content) = block_content.strip_prefix("for ") {
            let (ast, new_i) = self.parse_loop_block(content, i, depth)?;
            Ok(BlockTagResult::Parsed(ast, new_i))
        } else if block_content == "endif"
            || block_content == "endfor"
            || block_content == "elif"
            || block_content.starts_with("elif ")
            || block_content == "else"
        {
            Ok(BlockTagResult::EndTag)
        } else {
            Ok(BlockTagResult::Skip(block_end + 2))
        }
    }

    /// Parse an expression tag like {{ variable }}
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if expression tag parsing fails
    pub(crate) fn parse_expression_tag(
        &self,
        chars: &[char],
        i: usize,
        depth: usize,
    ) -> TemplateResult<(TemplateAst, usize)> {
        let expr_start = i + 2;
        let mut expr_end = expr_start;
        let mut brace_count = 0;

        while expr_end < chars.len() {
            if expr_end + 1 < chars.len() && chars[expr_end] == '}' && chars[expr_end + 1] == '}' {
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

        if expr_end >= chars.len() {
            return Err(TemplateError::ParseError {
                message: "Unclosed expression".to_string(),
            });
        }

        let expr_content: String = chars[expr_start..expr_end].iter().collect();
        let ast_node = self.parse_variable_or_expression(&expr_content, depth + 1)?;
        Ok((ast_node, expr_end + 2))
    }

    /// Parse a conditional block (if/elif/else/endif)
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if conditional block parsing fails
    pub(crate) fn parse_conditional_block(
        &self,
        content: &str,
        start_pos: usize,
        depth: usize,
    ) -> TemplateResult<(TemplateAst, usize)> {
        if depth > self.config.max_depth {
            return Err(TemplateError::ParseError {
                message: "Maximum parsing depth exceeded".to_string(),
            });
        }

        // Extract condition from {% if condition %}
        let chars: Vec<char> = content.chars().collect();
        let mut block_start = start_pos + 2; // skip {%
        while block_start < chars.len() && (chars[block_start] == ' ' || chars[block_start] == '\t')
        {
            block_start += 1;
        }

        // Find %}
        let mut block_end = block_start;
        while block_end + 1 < chars.len() {
            if chars[block_end] == '%' && chars[block_end + 1] == '}' {
                break;
            }
            block_end += 1;
        }

        let block_content: String = chars[block_start..block_end].iter().collect();
        let condition_str = block_content
            .strip_prefix("if ")
            .ok_or_else(|| TemplateError::ParseError {
                message: "Invalid if block syntax".to_string(),
            })?
            .trim();

        let condition = self.parse_expression(condition_str, depth + 1)?;

        // Find the body until endif/elif/else
        let body_start = block_end + 2;
        let (true_body, next_tag_pos, next_tag) =
            Self::find_block_end(content, body_start, &["endif", "elif", "else"])?;

        let if_true = self.parse_with_depth(&true_body, depth + 1)?;

        // Handle elif/else branches
        let (if_false, final_pos) = if next_tag.starts_with("elif") {
            // Parse elif as nested conditional
            let (elif_ast, end_pos) = self.parse_conditional_block(content, next_tag_pos, depth)?;
            (Some(Arc::new(elif_ast)), end_pos)
        } else if next_tag == "else" {
            // Find else body until endif
            let else_start = next_tag_pos;
            let chars_at: Vec<char> = content[else_start..].chars().collect();
            let mut i = 0;
            // Skip {% else %}
            while i + 1 < chars_at.len() {
                if chars_at[i] == '%' && chars_at[i + 1] == '}' {
                    i += 2;
                    break;
                }
                i += 1;
            }

            let (else_body, endif_pos, _) =
                Self::find_block_end(content, else_start + i, &["endif"])?;
            let else_ast = self.parse_with_depth(&else_body, depth + 1)?;
            (Some(Arc::new(else_ast)), endif_pos)
        } else {
            // endif - no else branch
            (None, next_tag_pos)
        };

        Ok((
            TemplateAst::Conditional {
                condition: Arc::new(condition),
                if_true: Arc::new(if_true),
                if_false,
            },
            final_pos,
        ))
    }

    /// Parse a loop block (for/endfor)
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if loop block parsing fails
    pub(crate) fn parse_loop_block(
        &self,
        content: &str,
        start_pos: usize,
        depth: usize,
    ) -> TemplateResult<(TemplateAst, usize)> {
        if depth > self.config.max_depth {
            return Err(TemplateError::ParseError {
                message: "Maximum parsing depth exceeded".to_string(),
            });
        }

        // Extract loop variable and iterable from {% for var in items %}
        let chars: Vec<char> = content.chars().collect();
        let mut block_start = start_pos + 2; // skip {%
        while block_start < chars.len() && (chars[block_start] == ' ' || chars[block_start] == '\t')
        {
            block_start += 1;
        }

        // Find %}
        let mut block_end = block_start;
        while block_end + 1 < chars.len() {
            if chars[block_end] == '%' && chars[block_end + 1] == '}' {
                break;
            }
            block_end += 1;
        }

        let block_content: String = chars[block_start..block_end].iter().collect();
        let loop_str = block_content
            .strip_prefix("for ")
            .ok_or_else(|| TemplateError::ParseError {
                message: "Invalid for block syntax".to_string(),
            })?
            .trim();

        // Parse "var in items"
        let parts: Vec<&str> = loop_str.split(" in ").collect();
        if parts.len() != 2 {
            return Err(TemplateError::ParseError {
                message: "Invalid for loop syntax, expected 'for var in items'".to_string(),
            });
        }

        let var_name = parts[0].trim();
        let iterable = parts[1].trim();

        // Find the body until endfor
        let body_start = block_end + 2;
        let (loop_body_str, endfor_pos, _) =
            Self::find_block_end(content, body_start, &["endfor"])?;

        let loop_body = self.parse_with_depth(&loop_body_str, depth + 1)?;

        Ok((
            TemplateAst::Loop {
                variable: var_name.to_string(),
                iterable: Arc::new(TemplateAst::Variable(iterable.to_string())),
                body: Arc::new(loop_body),
            },
            endfor_pos,
        ))
    }

    /// Find the end of a block, returning the body content, end position, and end tag name
    ///
    /// # Errors
    ///
    /// Returns `TemplateError` if block end is not found
    fn find_block_end(
        content: &str,
        start: usize,
        end_tags: &[&str],
    ) -> TemplateResult<(String, usize, String)> {
        let chars: Vec<char> = content[start..].chars().collect();
        let mut i = 0;
        let mut depth = 0;
        let mut body = String::new();

        while i < chars.len() {
            // Check for {% tag %}
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '%' {
                let tag_start = i + 2;
                let mut tag_end = tag_start;

                // Find %}
                while tag_end + 1 < chars.len() {
                    if chars[tag_end] == '%' && chars[tag_end + 1] == '}' {
                        break;
                    }
                    tag_end += 1;
                }

                if tag_end + 1 < chars.len() {
                    let tag_content: String = chars[tag_start..tag_end].iter().collect();
                    let tag_content = tag_content.trim();

                    // Check if this is a nested block start
                    if tag_content.starts_with("if ") || tag_content.starts_with("for ") {
                        depth += 1;
                        // Add to body
                        for ch in chars.iter().take(tag_end + 2).skip(i) {
                            body.push(*ch);
                        }
                        i = tag_end + 2;
                        continue;
                    }

                    // Check if this is an end tag
                    for end_tag in end_tags {
                        if (tag_content == *end_tag
                            || tag_content.starts_with(&format!("{end_tag} ")))
                            && depth == 0
                        {
                            // Found the matching end tag
                            return Ok((body, start + i, tag_content.to_string()));
                        }
                    }

                    // Check if this closes a nested block
                    if (tag_content == "endif" || tag_content == "endfor") && depth > 0 {
                        depth -= 1;
                        // Add to body
                        for ch in chars.iter().take(tag_end + 2).skip(i) {
                            body.push(*ch);
                        }
                        i = tag_end + 2;
                        continue;
                    }

                    // Other tag, add to body
                    for ch in chars.iter().take(tag_end + 2).skip(i) {
                        body.push(*ch);
                    }
                    i = tag_end + 2;
                    continue;
                }
            }

            body.push(chars[i]);
            i += 1;
        }

        Err(TemplateError::ParseError {
            message: format!("Unclosed block, expected one of: {end_tags:?}"),
        })
    }
}
