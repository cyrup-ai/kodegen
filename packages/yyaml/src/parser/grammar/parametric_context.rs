//! Parametric context for YAML 1.2 parsing

use crate::error::ScanError;

use super::context_types::{YamlContext, ParseContext, ChompingMode};

/// Tracks parametric context during parsing - integrates with existing indentation system
#[derive(Debug, Clone)]
pub struct ParametricContext {
    /// Context stack for YAML 1.2 parametric productions
    pub context_stack: Vec<YamlContext>,
    pub current_context: YamlContext,
    pub chomping_mode: Option<ChompingMode>,
    // REUSE existing indentation system - DO NOT DUPLICATE
    pub indentation: crate::parser::indentation::IndentationStateMachine,
    // ADD: Track recursion depth
    pub recursion_depth: usize,
    pub max_depth: usize,
}

impl ParametricContext {
    #[must_use]
    pub fn new() -> Self {
        Self {
            context_stack: vec![YamlContext::BlockOut], // Document level starts BLOCK-OUT
            current_context: YamlContext::BlockOut,
            chomping_mode: None,
            // REUSE existing IndentationStateMachine from parser/indentation.rs
            indentation: crate::parser::indentation::IndentationStateMachine::new(),
            recursion_depth: 0,
            max_depth: 100, // Reasonable default
        }
    }

    pub fn push_context(&mut self, context: YamlContext, indent: i32) {
        self.context_stack.push(self.current_context);
        self.current_context = context;

        // Use existing indentation system
        let is_sequence = matches!(context, YamlContext::BlockIn);
        self.indentation.push_indent(indent as usize, is_sequence);
    }

    pub fn pop_context(&mut self) {
        if let Some(context) = self.context_stack.pop() {
            self.current_context = context;
            self.indentation.pop_indent();
        }
    }

    /// Get current indentation from existing system
    #[must_use]
    pub fn current_indent(&self) -> i32 {
        self.indentation.current_indent() as i32
    }

    /// Calculate n+m indentation for block collections per YAML 1.2 spec
    #[must_use]
    pub const fn calculate_block_indent(&self, base: i32, offset: i32) -> i32 {
        base + offset
    }

    /// Set chomping mode for block scalars
    pub const fn set_chomping_mode(&mut self, mode: ChompingMode) {
        self.chomping_mode = Some(mode);
    }

    /// Clear chomping mode after processing block scalar
    pub const fn clear_chomping_mode(&mut self) {
        self.chomping_mode = None;
    }

    /// Convert YAML 1.2 Context to existing ParseContext for backward compatibility
    #[must_use]
    pub fn to_parse_context(&self) -> ParseContext {
        match self.current_context {
            YamlContext::BlockIn => ParseContext::BlockIn(self.current_indent() as usize),
            YamlContext::BlockOut => ParseContext::Document,
            YamlContext::BlockKey => ParseContext::BlockKey,
            YamlContext::FlowIn => ParseContext::FlowIn(self.context_stack.len()),
            YamlContext::FlowOut => ParseContext::FlowValue,
            YamlContext::FlowKey => ParseContext::FlowKey,
        }
    }

    pub fn increment_depth(&mut self) -> Result<(), ScanError> {
        self.recursion_depth += 1;
        if self.recursion_depth > self.max_depth {
            Err(ScanError::new(
                crate::error::Marker::default(),
                "maximum recursion depth exceeded",
            ))
        } else {
            Ok(())
        }
    }

    pub const fn decrement_depth(&mut self) {
        if self.recursion_depth > 0 {
            self.recursion_depth -= 1;
        }
    }
}

impl Default for ParametricContext {
    fn default() -> Self {
        Self::new()
    }
}
