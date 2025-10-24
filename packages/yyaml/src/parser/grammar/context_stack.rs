//! Context stack for tracking nested parsing contexts

use super::context_types::ParseContext;

#[derive(Debug, Clone)]
pub struct ContextStack {
    stack: Vec<ParseContext>,
}

impl Default for ContextStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextStack {
    /// Create new context stack starting with document context
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            stack: vec![ParseContext::Document],
        }
    }

    /// Get current context
    #[inline]
    #[must_use]
    pub fn current(&self) -> &ParseContext {
        self.stack.last().unwrap_or(&ParseContext::Document)
    }

    /// Push new context
    #[inline]
    pub fn push(&mut self, context: ParseContext) {
        self.stack.push(context);
    }

    /// Pop context
    pub fn pop(&mut self) -> Option<ParseContext> {
        if self.stack.len() > 1 {
            self.stack.pop()
        } else {
            None
        }
    }

    /// Get nesting depth
    #[inline]
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Check if we're in a flow context
    #[inline]
    #[must_use]
    pub fn in_flow_context(&self) -> bool {
        matches!(
            self.current(),
            ParseContext::FlowIn(_) | ParseContext::FlowKey | ParseContext::FlowValue
        )
    }

    /// Check if we're in a block context
    #[inline]
    #[must_use]
    pub fn in_block_context(&self) -> bool {
        matches!(
            self.current(),
            ParseContext::BlockIn(_) | ParseContext::BlockKey | ParseContext::BlockValue
        )
    }

    /// Get current indentation level (for block contexts)
    #[must_use]
    pub fn current_indent(&self) -> Option<usize> {
        match self.current() {
            ParseContext::BlockIn(indent) => Some(*indent),
            _ => None,
        }
    }

    /// Get current flow level (for flow contexts)
    #[must_use]
    pub fn current_flow_level(&self) -> Option<usize> {
        match self.current() {
            ParseContext::FlowIn(level) => Some(*level),
            _ => None,
        }
    }
}
