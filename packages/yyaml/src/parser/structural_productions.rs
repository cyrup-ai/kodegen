//! YAML 1.2 Structural Productions [63-81] - Concrete Implementation
//!
//! This module implements production rule functions that use the existing
//! parametric production definitions, context system, and infrastructure.

use crate::error::ScanError;
use crate::parser::character_productions::CharacterProductions;
use crate::parser::grammar::{ChompingMode, YamlContext, ParametricContext};
use crate::scanner::state::ScannerState;
use crate::scanner::utils::{consume_line_break, skip_whitespace_and_comments};

/// Structural productions implementation using existing infrastructure
pub struct StructuralProductions;

impl StructuralProductions {
    /// [63] s-indent(n) ::= s-space × n
    /// Uses existing ParametricContext for validation
    pub fn validate_exact_indent<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &ParametricContext,
        n: i32,
    ) -> Result<bool, ScanError> {
        // First check context for cached indentation
        let current_indent = context.current_indent();
        if current_indent != n {
            return Ok(false);
        }

        // Validate that the next n characters are spaces
        for i in 0..n {
            match state.peek_char_at_raw(i as usize) {
                Some(' ') => continue,
                Some(_) => return Ok(false),
                None => {
                    return Err(ScanError::new(
                        state.mark(),
                        &format!("insufficient input for {}-space indentation", n),
                    ));
                }
            }
        }
        Ok(true)
    }

    /// [64] s-indent(<n) ::= s-space × m /* Where m < n */
    /// Returns actual indentation found (less than n)
    pub fn validate_indent_less_than<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &ParametricContext,
        n: i32,
    ) -> Result<i32, ScanError> {
        let current_indent = context.current_indent();
        if current_indent < n {
            Ok(current_indent)
        } else {
            Err(ScanError::new(
                state.mark(),
                &format!("indentation {} not less than {}", current_indent, n),
            ))
        }
    }

    /// [65] s-indent(≤n) ::= s-space × m /* Where m ≤ n */
    pub fn validate_indent_less_equal<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &ParametricContext,
        n: i32,
    ) -> Result<i32, ScanError> {
        let current_indent = context.current_indent();
        if current_indent <= n {
            Ok(current_indent)
        } else {
            Err(ScanError::new(
                state.mark(),
                &format!("indentation {} not ≤ {}", current_indent, n),
            ))
        }
    }

    /// [67] s-line-prefix(n,c) - Context-aware line prefix
    /// DELEGATES to existing context and indentation systems
    pub fn process_line_prefix<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &mut ParametricContext,
        n: i32,
    ) -> Result<(), ScanError> {
        match context.current_context {
            YamlContext::BlockOut | YamlContext::BlockIn => {
                Self::process_block_line_prefix(state, context, n)
            }
            YamlContext::FlowOut | YamlContext::FlowIn => Self::process_flow_line_prefix(state, context, n),
            _ => Ok(()),
        }
    }

    /// [68] s-block-line-prefix(n) ::= s-indent(n)
    pub fn process_block_line_prefix<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &ParametricContext,
        n: i32,
    ) -> Result<(), ScanError> {
        // USE existing indentation validation
        if !Self::validate_exact_indent(state, context, n)? {
            return Err(ScanError::new(
                state.mark(),
                &format!("expected {} spaces of block indentation", n),
            ));
        }
        Ok(())
    }

    /// [69] s-flow-line-prefix(n) ::= s-indent(n) s-separate-in-line?
    pub fn process_flow_line_prefix<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &ParametricContext,
        n: i32,
    ) -> Result<(), ScanError> {
        Self::process_block_line_prefix(state, context, n)?;
        // USE existing whitespace skipping
        while let Ok(ch) = state.peek_char() {
            if CharacterProductions::is_white(ch) {
                state.consume_char()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// [70] l-empty(n,c) ::= ( s-line-prefix(n,c) | s-indent(<n) ) b-as-line-feed
    pub fn process_empty_line<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &mut ParametricContext,
        n: i32,
    ) -> Result<bool, ScanError> {
        // Try line prefix first
        if Self::process_line_prefix(state, context, n).is_ok() {
            return Self::consume_line_break_if_present(state);
        }

        // Try less indentation
        if Self::validate_indent_less_than(state, context, n).is_ok() {
            return Self::consume_line_break_if_present(state);
        }

        Ok(false)
    }

    /// [71-74] Line folding productions - USE existing scalar folding
    #[must_use] 
    pub fn apply_line_folding(
        lines: &[String],
        chomping: ChompingMode,
        literal_style: bool,
    ) -> String {
        // DELEGATE to existing line folding in scalars.rs
        crate::scanner::scalars::apply_block_scalar_folding(lines, chomping, literal_style)
    }

    /// [75] c-nb-comment-text ::= "#" nb-char*
    /// REUSE existing comment parsing
    pub fn parse_comment_text<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
    ) -> Result<String, ScanError> {
        if state.peek_char()? != '#' {
            return Err(ScanError::new(state.mark(), "expected comment marker '#'"));
        }

        state.consume_char()?; // Consume '#'
        let mut comment = String::new();

        while let Ok(ch) = state.peek_char() {
            if CharacterProductions::is_ns_char(ch) {
                comment.push(state.consume_char()?);
            } else {
                break;
            }
        }

        Ok(comment)
    }

    /// [76-79] Comment productions - USE existing comment utilities
    pub fn skip_comment_lines<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
    ) -> Result<Vec<String>, ScanError> {
        let mut comments = Vec::new();

        while let Ok('#') = state.peek_char() {
            let comment = Self::parse_comment_text(state)?;
            comments.push(comment);

            // USE existing line break consumption
            if matches!(state.peek_char(), Ok('\n') | Ok('\r')) {
                consume_line_break(state)?;
            } else {
                break;
            }
        }

        Ok(comments)
    }

    /// [80] s-separate(n,c) - Context-aware separation
    pub fn process_separation<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &mut ParametricContext,
        n: i32,
    ) -> Result<(), ScanError> {
        match context.current_context {
            YamlContext::BlockKey | YamlContext::FlowKey => {
                // USE existing whitespace skipping
                while let Ok(ch) = state.peek_char() {
                    if CharacterProductions::is_white(ch) {
                        state.consume_char()?;
                    } else {
                        break;
                    }
                }
                Ok(())
            }
            _ => Self::process_separation_lines(state, context, n),
        }
    }

    /// [81] s-separate-lines(n) ::= ( s-l-comments s-flow-line-prefix(n) ) | s-separate-in-line
    pub fn process_separation_lines<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
        context: &mut ParametricContext,
        n: i32,
    ) -> Result<(), ScanError> {
        // Try comments + flow line prefix
        let _comments = Self::skip_comment_lines(state)?;

        if Self::process_flow_line_prefix(state, context, n).is_ok() {
            return Ok(());
        }

        // Fallback to inline separation - USE existing function
        skip_whitespace_and_comments(state)
    }

    /// [66] s-separate-in-line ::= s-white+ | <start-of-line>
    /// Handles in-line separation between tokens
    pub fn process_separate_in_line<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
    ) -> Result<(), ScanError> {
        // Check if at start of line
        if state.at_line_start() {
            return Ok(());
        }

        // Otherwise require at least one white space character
        let mut found_white = false;
        while let Ok(ch) = state.peek_char() {
            if CharacterProductions::is_white(ch) {
                state.consume_char()?;
                found_white = true;
            } else {
                break;
            }
        }

        if !found_white {
            return Err(ScanError::new(
                state.mark(),
                "expected whitespace or start of line for in-line separation",
            ));
        }

        Ok(())
    }

    // Helper functions using existing infrastructure

    fn consume_line_break_if_present<T: Iterator<Item = char>>(
        state: &mut ScannerState<T>,
    ) -> Result<bool, ScanError> {
        match state.peek_char() {
            Ok('\n') | Ok('\r') => {
                consume_line_break(state)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

/// Integration with existing scanner - ADD methods to ScannerState
impl<T: Iterator<Item = char>> ScannerState<T> {
    /// Process structural separation using existing infrastructure
    pub fn process_structural_separation(
        &mut self,
        context: &mut ParametricContext,
        n: i32,
    ) -> Result<(), ScanError> {
        StructuralProductions::process_separation(self, context, n)
    }

    /// Validate structural indentation using existing system
    pub fn validate_structural_indent(
        &mut self,
        context: &ParametricContext,
        n: i32,
    ) -> Result<bool, ScanError> {
        StructuralProductions::validate_exact_indent(self, context, n)
    }

    /// Skip comments using existing utilities
    pub fn skip_structural_comments(&mut self) -> Result<Vec<String>, ScanError> {
        StructuralProductions::skip_comment_lines(self)
    }
}
