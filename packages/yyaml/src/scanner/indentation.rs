//! Indentation tracking and block structure management
//!
//! This module provides efficient indentation level tracking for YAML block
//! structure with proper validation and context management.

use crate::error::{Marker, ScanError};
use crate::scanner::state::ScannerState;

/// Indentation context for block structure
#[derive(Debug, Clone)]
pub struct IndentationTracker {
    /// Stack of indentation levels
    levels: Vec<IndentLevel>,
    /// Current base indentation
    current_base: i32,
    /// Minimum allowed indentation
    min_indent: i32,
}

/// Single indentation level record
#[derive(Debug, Clone, Copy)]
struct IndentLevel {
    /// Column position of this indentation level
    column: i32,
    /// Type of block at this level
    block_type: BlockType,
    /// Whether simple keys are allowed at this level
    simple_key_allowed: bool,
}

/// Types of YAML block structures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    /// Block sequence (- items)
    Sequence,
    /// Block mapping (key: value)
    Mapping,
    /// Root document level
    Root,
    /// Plain scalar continuation
    Scalar,
}

impl IndentationTracker {
    /// Create new indentation tracker
    #[inline]
    #[must_use] 
    pub fn new() -> Self {
        Self {
            levels: Vec::with_capacity(16),
            current_base: -1,
            min_indent: 0,
        }
    }

    /// Reset tracker for new document
    #[inline]
    pub fn reset(&mut self) {
        self.levels.clear();
        self.current_base = -1;
        self.min_indent = 0;
    }

    /// Get current indentation level
    #[inline]
    #[must_use] 
    pub const fn current_level(&self) -> i32 {
        self.current_base
    }

    /// Check if at root level
    #[inline]
    #[must_use] 
    pub const fn at_root_level(&self) -> bool {
        self.levels.is_empty()
    }

    /// Get current block type
    #[inline]
    #[must_use] 
    pub fn current_block_type(&self) -> BlockType {
        self.levels
            .last()
            .map(|level| level.block_type)
            .unwrap_or(BlockType::Root)
    }

    /// Push new indentation level
    #[inline]
    pub fn push_level(&mut self, column: i32, block_type: BlockType) -> Result<(), ScanError> {
        // Validate indentation increase
        if column <= self.current_base {
            return Err(ScanError::new(
                Marker::at(0, 1, column as usize),
                "indentation must increase for nested blocks",
            ));
        }

        // Check maximum nesting depth
        if self.levels.len() >= 128 {
            return Err(ScanError::new(
                Marker::at(0, 1, column as usize),
                "maximum nesting depth exceeded (128 levels)",
            ));
        }

        let level = IndentLevel {
            column,
            block_type,
            simple_key_allowed: true,
        };

        self.levels.push(level);
        self.current_base = column;

        Ok(())
    }

    /// Pop indentation levels until we reach target column or below
    #[inline]
    pub fn pop_to_level(&mut self, target_column: i32) -> Vec<BlockType> {
        let mut popped_types = Vec::new();

        while let Some(level) = self.levels.last() {
            if level.column <= target_column {
                break;
            }

            // Safe to unwrap here since we verified levels.last() exists above,
            // but using explicit pattern match for artisan-quality production code
            if let Some(popped) = self.levels.pop() {
                popped_types.push(popped.block_type);
            }
        }

        self.current_base = self.levels.last().map(|level| level.column).unwrap_or(-1);

        popped_types
    }

    /// Check if simple key is allowed at current level
    #[inline]
    #[must_use] 
    pub fn simple_key_allowed(&self) -> bool {
        self.levels
            .last()
            .map(|level| level.simple_key_allowed)
            .unwrap_or(true)
    }

    /// Set simple key allowed flag for current level
    #[inline]
    pub fn set_simple_key_allowed(&mut self, allowed: bool) {
        if let Some(level) = self.levels.last_mut() {
            level.simple_key_allowed = allowed;
        }
    }

    /// Check if column is valid for current context
    #[inline]
    #[must_use] 
    pub const fn is_valid_column(&self, column: i32, block_type: BlockType) -> bool {
        match block_type {
            BlockType::Sequence | BlockType::Mapping => {
                // Block collections must be more indented than parent
                column > self.current_base
            }
            BlockType::Scalar => {
                // Scalar content can be at same level or more indented
                column >= self.current_base
            }
            BlockType::Root => {
                // Root level content starts at column 0
                column == 0
            }
        }
    }

    /// Get depth of nesting
    #[inline]
    #[must_use] 
    pub const fn depth(&self) -> usize {
        self.levels.len()
    }

    /// Get minimum required indentation for new block
    #[inline]
    #[must_use] 
    pub const fn min_indent_for_block(&self, block_type: BlockType) -> i32 {
        match block_type {
            BlockType::Root => 0,
            BlockType::Sequence | BlockType::Mapping => self.current_base + 1,
            BlockType::Scalar => self.current_base,
        }
    }
}

impl Default for IndentationTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Count leading whitespace at current position
#[inline]
pub fn count_leading_whitespace<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<(usize, bool), ScanError> {
    let mut count = 0;
    let has_tabs = false;

    while let Ok(ch) = state.peek_char() {
        match ch {
            ' ' => {
                state.consume_char()?;
                count += 1;
            }
            '\t' => {
                // YAML 1.2 spec: Only spaces are allowed for indentation
                return Err(ScanError::new(
                    state.mark(),
                    "tabs are not allowed in YAML indentation, use spaces only",
                ));
            }
            _ => break,
        }
    }

    Ok((count, has_tabs))
}

/// Detect indentation level of current line
#[inline]
pub fn detect_line_indentation<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<LineIndentation, ScanError> {
    let start_mark = state.mark();
    let (spaces, has_tabs) = count_leading_whitespace(state)?;

    // Check what follows the indentation
    let content_type = match state.peek_char() {
        Err(_) => ContentType::EndOfFile,
        Ok('\n') | Ok('\r') => ContentType::EmptyLine,
        Ok('#') => ContentType::Comment,
        Ok('-') => {
            // Could be block entry or document start
            if state.check_document_start()? {
                ContentType::DocumentStart
            } else if state.check_block_entry()? {
                ContentType::BlockEntry
            } else {
                ContentType::Content
            }
        }
        Ok('.') => {
            if state.check_document_end()? {
                ContentType::DocumentEnd
            } else {
                ContentType::Content
            }
        }
        Ok(_) => ContentType::Content,
    };

    Ok(LineIndentation {
        column: spaces,
        has_tabs,
        content_type,
        position: start_mark,
    })
}

/// Information about line indentation
#[derive(Debug, Clone)]
pub struct LineIndentation {
    /// Column position after indentation
    pub column: usize,
    /// Whether tabs were used in indentation
    pub has_tabs: bool,
    /// Type of content following indentation
    pub content_type: ContentType,
    /// Position at start of line
    pub position: Marker,
}

/// Types of content that can follow indentation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// Regular YAML content
    Content,
    /// Block entry indicator (-)
    BlockEntry,
    /// Comment line (#)
    Comment,
    /// Empty line
    EmptyLine,
    /// Document start marker (---)
    DocumentStart,
    /// Document end marker (...)
    DocumentEnd,
    /// End of file
    EndOfFile,
}

/// Validate indentation consistency
pub fn validate_indentation_consistency(
    current: &LineIndentation,
    previous: &LineIndentation,
) -> Result<(), ScanError> {
    // Mixed tabs and spaces are problematic
    if current.has_tabs && previous.has_tabs {
        // Both use tabs - this is fine
        return Ok(());
    }

    if !current.has_tabs && !previous.has_tabs {
        // Both use spaces - this is fine
        return Ok(());
    }

    // Mixed tabs and spaces
    Err(ScanError::new(
        current.position,
        "inconsistent use of tabs and spaces for indentation",
    ))
}

/// Calculate effective indentation considering tab stops
#[inline]
#[must_use] 
pub const fn effective_indentation(spaces: usize, has_tabs: bool, _tab_width: usize) -> usize {
    if has_tabs {
        // This is approximation - actual calculation depends on where tabs occur
        spaces
    } else {
        spaces
    }
}

/// Check if indentation is valid for block context
pub fn validate_block_indentation(
    indentation: &LineIndentation,
    tracker: &IndentationTracker,
    expected_block_type: BlockType,
) -> Result<(), ScanError> {
    let column = indentation.column as i32;
    let _min_required = tracker.min_indent_for_block(expected_block_type);

    match expected_block_type {
        BlockType::Root => {
            if column != 0 {
                return Err(ScanError::new(
                    indentation.position,
                    "root level content must start at column 0",
                ));
            }
        }
        BlockType::Sequence | BlockType::Mapping => {
            if column <= tracker.current_level() {
                return Err(ScanError::new(
                    indentation.position,
                    &format!(
                        "block collections must be indented more than parent level ({})",
                        tracker.current_level()
                    ),
                ));
            }
        }
        BlockType::Scalar => {
            if column < tracker.current_level() {
                return Err(ScanError::new(
                    indentation.position,
                    &format!(
                        "scalar content must be indented at least as much as parent level ({})",
                        tracker.current_level()
                    ),
                ));
            }
        }
    }

    Ok(())
}

/// Determine required block end tokens from indentation change
pub fn determine_block_ends(
    current_column: i32,
    tracker: &mut IndentationTracker,
) -> Vec<BlockType> {
    if current_column < tracker.current_level() {
        tracker.pop_to_level(current_column)
    } else {
        Vec::new()
    }
}

/// Skip empty lines and comments, returning indentation of next content line
pub fn skip_to_content_line<T: Iterator<Item = char>>(
    state: &mut ScannerState<T>,
) -> Result<Option<LineIndentation>, ScanError> {
    loop {
        let indentation = detect_line_indentation(state)?;

        match indentation.content_type {
            ContentType::EmptyLine => {
                // Skip empty line
                while matches!(state.peek_char(), Ok('\n') | Ok('\r')) {
                    state.consume_char()?;
                }
                continue;
            }
            ContentType::Comment => {
                // Skip comment line
                while let Ok(ch) = state.peek_char() {
                    state.consume_char()?;
                    if matches!(ch, '\n' | '\r') {
                        break;
                    }
                }
                continue;
            }
            ContentType::EndOfFile => return Ok(None),
            _ => return Ok(Some(indentation)),
        }
    }
}
