//! Internal utility functions for parsing

use super::super::core::{TemplateAst, TemplateError, TemplateResult};

/// Result of parsing a block tag
pub(crate) enum BlockTagResult {
    /// Successfully parsed a block tag with the AST and new position
    Parsed(TemplateAst, usize),
    /// Encountered an end tag (endif, endfor, etc.)
    EndTag,
    /// Skip this tag and move to the new position
    Skip(usize),
}

/// Find the position of an operator in content, respecting parenthesis depth
pub(crate) fn find_operator(content: &str, operators: &[&str]) -> Option<usize> {
    let chars: Vec<char> = content.chars().collect();
    let mut i: usize = 0;
    let mut paren_depth: usize = 0;
    let mut bracket_depth: usize = 0;

    while i < chars.len() {
        match chars[i] {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }

        if paren_depth == 0 && bracket_depth == 0 {
            for op in operators {
                if i + op.len() <= chars.len() {
                    let substr: String = chars[i..i + op.len()].iter().collect();
                    if substr == *op {
                        return Some(i);
                    }
                }
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    None
}

/// Extract operator and split content into left, operator, right parts
pub(crate) fn extract_operator(
    content: &str,
    pos: usize,
    operators: &[&str],
) -> TemplateResult<(String, String, String)> {
    let chars: Vec<char> = content.chars().collect();

    for op in operators {
        if pos + op.len() <= chars.len() {
            let substr: String = chars[pos..pos + op.len()].iter().collect();
            if substr == *op {
                let left: String = chars[..pos].iter().collect();
                let right: String = chars[pos + op.len()..].iter().collect();
                return Ok((left, (*op).to_string(), right));
            }
        }
    }

    Err(TemplateError::ParseError {
        message: "Operator not found at position".to_string(),
    })
}
