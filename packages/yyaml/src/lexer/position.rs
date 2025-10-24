//! Precise source location tracking for YAML tokens
//!
//! This module provides high-performance position tracking with minimal overhead
//! for accurate error reporting and debugging.

/// Precise position in source text
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)  
    pub column: usize,
    /// Byte offset from start of input (0-based)
    pub byte_offset: usize,
}

impl Position {
    /// Create a new position
    #[inline]
    #[must_use] 
    pub const fn new(line: usize, column: usize, byte_offset: usize) -> Self {
        Self {
            line,
            column,
            byte_offset,
        }
    }

    /// Create position at start of input
    #[inline]
    #[must_use] 
    pub const fn start() -> Self {
        Self::new(1, 1, 0)
    }

    /// Check if this is the start position
    #[inline]
    #[must_use] 
    pub const fn is_start(&self) -> bool {
        self.line == 1 && self.column == 1 && self.byte_offset == 0
    }

    /// Advance position by one character
    #[inline]
    pub const fn advance_char(&mut self, ch: char) {
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.byte_offset += ch.len_utf8();
    }

    /// Advance position by multiple characters
    #[inline]
    pub fn advance_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.advance_char(ch);
        }
    }

    /// Create a span from this position to another
    #[inline]
    #[must_use] 
    pub const fn span_to(self, end: Self) -> Span {
        Span::new(self, end)
    }
}

impl Default for Position {
    #[inline]
    fn default() -> Self {
        Self::start()
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A span of text in the source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    /// Create a new span
    #[inline]
    #[must_use] 
    pub const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Create a span covering a single position
    #[inline]
    #[must_use] 
    pub const fn point(pos: Position) -> Self {
        Self::new(pos, pos)
    }

    /// Get the length of this span in bytes
    #[inline]
    #[must_use] 
    pub const fn byte_len(&self) -> usize {
        self.end.byte_offset.saturating_sub(self.start.byte_offset)
    }

    /// Check if this span contains a position
    #[inline]
    #[must_use] 
    pub fn contains(&self, pos: Position) -> bool {
        pos >= self.start && pos <= self.end
    }

    /// Check if this span overlaps with another
    #[inline]
    #[must_use] 
    pub fn overlaps(&self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    /// Merge this span with another
    #[inline]
    #[must_use] 
    pub fn merge(self, other: Self) -> Self {
        Self::new(
            std::cmp::min(self.start, other.start),
            std::cmp::max(self.end, other.end),
        )
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

/// High-performance position tracker for lexical analysis
#[derive(Debug, Clone)]
pub struct PositionTracker {
    current: Position,
    mark_stack: Vec<Position>,
}

impl PositionTracker {
    /// Create a new position tracker
    #[inline]
    #[must_use] 
    pub const fn new() -> Self {
        Self {
            current: Position::start(),
            mark_stack: Vec::new(),
        }
    }

    /// Get the current position
    #[inline]
    #[must_use] 
    pub const fn current(&self) -> Position {
        self.current
    }

    /// Advance by one character
    #[inline]
    pub const fn advance_char(&mut self, ch: char) {
        self.current.advance_char(ch);
    }

    /// Advance by a string
    #[inline]
    pub fn advance_str(&mut self, s: &str) {
        self.current.advance_str(s);
    }

    /// Advance by a specific number of bytes
    #[inline]
    pub const fn advance_bytes(&mut self, bytes: usize) {
        self.current.byte_offset += bytes;
        self.current.column += bytes; // Assumes single-byte characters
    }

    /// Mark the current position for potential backtracking
    #[inline]
    pub fn mark(&mut self) {
        self.mark_stack.push(self.current);
    }

    /// Return to the last marked position
    #[inline]
    pub fn reset(&mut self) -> bool {
        if let Some(pos) = self.mark_stack.pop() {
            self.current = pos;
            true
        } else {
            false
        }
    }

    /// Drop the last mark without resetting
    #[inline]
    pub fn drop_mark(&mut self) -> bool {
        self.mark_stack.pop().is_some()
    }

    /// Clear all marks
    #[inline]
    pub fn clear_marks(&mut self) {
        self.mark_stack.clear();
    }

    /// Get the number of active marks
    #[inline]
    #[must_use] 
    pub const fn mark_count(&self) -> usize {
        self.mark_stack.len()
    }

    /// Create a span from the last mark to current position
    #[inline]
    #[must_use] 
    pub fn span_from_mark(&self) -> Option<Span> {
        self.mark_stack
            .last()
            .map(|&start| Span::new(start, self.current))
    }

    /// Handle line ending normalization
    #[inline]
    pub fn handle_line_ending(&mut self, input: &str, offset: usize) -> usize {
        if let Some(ch) = input.chars().nth(offset) {
            match ch {
                '\n' => {
                    self.advance_char('\n');
                    1
                }
                '\r' => {
                    // Handle \r\n and standalone \r
                    if input.chars().nth(offset + 1) == Some('\n') {
                        self.advance_char('\n');
                        2
                    } else {
                        self.advance_char('\n');
                        1
                    }
                }
                _ => 0,
            }
        } else {
            0
        }
    }
}

impl Default for PositionTracker {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Utilities for working with line and column information
pub mod utils {
    use super::*;

    /// Calculate position for a byte offset in text
    #[must_use] 
    pub fn position_at_offset(text: &str, target_offset: usize) -> Position {
        let mut pos = Position::start();

        for (offset, ch) in text.char_indices() {
            if offset >= target_offset {
                break;
            }
            pos.advance_char(ch);
        }

        pos
    }

    /// Get the line containing a position
    #[must_use] 
    pub fn line_at_position(text: &str, position: Position) -> Option<&str> {
        let lines: Vec<&str> = text.lines().collect();
        if position.line > 0 && position.line <= lines.len() {
            Some(lines[position.line - 1])
        } else {
            None
        }
    }

    /// Extract text for a span
    #[must_use] 
    pub fn text_for_span(input: &str, span: Span) -> Option<&str> {
        let start_offset = span.start.byte_offset;
        let end_offset = span.end.byte_offset;

        if start_offset <= input.len() && end_offset <= input.len() && start_offset <= end_offset {
            Some(&input[start_offset..end_offset])
        } else {
            None
        }
    }

    /// Create a visual indicator for a position in text
    #[must_use] 
    pub fn position_indicator(text: &str, position: Position, width: usize) -> String {
        if let Some(line) = line_at_position(text, position) {
            let mut result = String::new();
            result.push_str(line);
            result.push('\n');

            // Add indicator pointing to the column
            let spaces = position.column.saturating_sub(1).min(width);
            result.push_str(&" ".repeat(spaces));
            result.push('^');

            result
        } else {
            format!("Position {}:{} (invalid)", position.line, position.column)
        }
    }
}
