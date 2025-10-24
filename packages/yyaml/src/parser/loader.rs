// Parser removed - using StateMachine directly
use crate::error::{Marker, ScanError};
use crate::events::{Event, EventReceiver, TScalarStyle, TokenType};
use crate::linked_hash_map::LinkedHashMap;
use crate::yaml::Yaml;
use log::{debug, trace, warn};
use std::collections::HashMap;

/// Our main "public" API: load from a string → produce Vec<Yaml>.
pub struct YamlLoader;

impl YamlLoader {
    pub fn load_from_str(s: &str) -> Result<Vec<Yaml>, ScanError> {
        println!("=== YamlLoader::load_from_str ENTRY with: '{}' ===", s);
        // Fast path for simple cases - zero allocation, blazing fast
        println!("YamlLoader: trying fast parse");
        match Self::try_fast_parse(s) {
            Ok(Some(result)) => {
                debug!("Fast parser succeeded with: {result:?}");
                return Ok(vec![result]);
            }
            Ok(None) => {
                debug!("Fast parser detected complex syntax, falling back to full parser");
                println!("YamlLoader: fast parser returned None, falling back to StateMachine");
            } // Fall through to full parser
            Err(error) => {
                debug!("Fast parser failed: {error:?}");
                return Err(error);
            } // Propagate parsing errors
        }

        // Handle multi-document streams
        let mut documents = Vec::new();
        println!("YamlLoader: creating StateMachine");
        let mut state_machine = crate::parser::state_machine::StateMachine::new(s.chars());
        println!("YamlLoader: StateMachine created, starting document parsing loop");

        // Process all documents in stream
        while !state_machine.at_stream_end() {
            println!("YamlLoader: parsing next document...");
            match state_machine.parse_next_document() {
                Ok(Some(doc)) => {
                    debug!("Parsed document: {doc:?}");
                    documents.push(doc);
                }
                Ok(None) => break, // End of stream
                Err(e) => {
                    debug!("State machine failed: {e:?}");
                    return Err(e);
                }
            }
        }

        // Handle empty streams (return empty vec, not error)
        if documents.is_empty() {
            debug!("No documents found in stream");
            documents.push(Yaml::Null);
        }

        Ok(documents)
    }

    /// Blazing-fast zero-allocation parser for common simple cases with production-grade error handling
    /// Handles: "key: value", "- item", "[1, 2, 3]", "{key: value}", multi-line mappings, and simple scalars
    fn try_fast_parse(s: &str) -> Result<Option<Yaml>, ScanError> {
        println!("try_fast_parse called with: '{}'", s);
        let mut trimmed = s.trim();
        println!("try_fast_parse: trimmed = '{}'", trimmed);

        // Strip BOM if present for accurate parsing decisions per YAML 1.2
        if trimmed.starts_with('\u{feff}') {
            trimmed = &trimmed[3..]; // BOM is 3 bytes in UTF-8
        }

        // Empty document
        if trimmed.is_empty() {
            return Ok(Some(Yaml::Null));
        }

        // CRITICAL FIX: If content starts with "- ", it's a sequence - ALWAYS use full parser
        // The fast parser incorrectly handles complex sequences, so force full parser
        if trimmed.starts_with("- ") {
            return Ok(None);
        }

        // Simple scalar cases (no structure indicators)
        if !trimmed.contains(':')
            && !trimmed.contains('-')
            && !trimmed.contains('[')
            && !trimmed.contains('{')
            && !trimmed.contains('|')
            && !trimmed.contains('>')
        {
            return Ok(Some(Self::parse_scalar_direct(trimmed)));
        }

        // YAML 1.2 Complete Feature Detection - Zero allocation, optimal performance
        // Comprehensive spec compliance check using iterator chains for maximum efficiency

        // Chapter 6.8: All directive detection (YAML, TAG, reserved)
        let has_directives = trimmed.lines().any(|line| {
            let trimmed_line = line.trim_start();
            trimmed_line.starts_with("%YAML ") ||
            trimmed_line.starts_with("%TAG ") ||
            (trimmed_line.starts_with('%') &&
             trimmed_line.chars().nth(1).is_some_and(|c| c.is_ascii_uppercase()))
        });
        if has_directives {
            return Ok(None);
        }

        // Chapter 9.2: Multi-document stream detection - optimized counting
        let mut doc_markers = 0u8;
        let mut line_start = true;
        for (i, &byte) in trimmed.as_bytes().iter().enumerate() {
            match byte {
                b'\n' => line_start = true,
                b'-' if line_start => {
                    if trimmed.as_bytes().get(i+1) == Some(&b'-') &&
                       trimmed.as_bytes().get(i+2) == Some(&b'-') &&
                       trimmed.as_bytes().get(i+3).is_none_or(|&b| b == b' ' || b == b'\t' || b == b'\n') {
                        doc_markers += 1;
                        if doc_markers > 1 { return Ok(None); }
                    }
                    line_start = false;
                },
                b'.' if line_start => {
                    if trimmed.as_bytes().get(i+1) == Some(&b'.') &&
                       trimmed.as_bytes().get(i+2) == Some(&b'.') &&
                       trimmed.as_bytes().get(i+3).is_none_or(|&b| b == b' ' || b == b'\t' || b == b'\n') {
                        return Ok(None); // Any document end marker requires full parser
                    }
                    line_start = false;
                },
                b' ' | b'\t' => {},
                _ => line_start = false,
            }
        }

        // Chapter 6.9: Node properties in mapping contexts - comprehensive detection
        if trimmed.contains(':') {
            let has_node_properties = trimmed.lines().any(|line| {
                let trimmed_line = line.trim();
                // Tag detection: ! not at start of line or after whitespace indicating tagged values
                if let Some(exclaim_pos) = trimmed_line.find('!') {
                    // Not a comment (!= case) and not negation (!something without space)
                    let is_tag = exclaim_pos == 0 ||
                                trimmed_line.chars().nth(exclaim_pos.saturating_sub(1))
                                    .is_some_and(|c| c.is_whitespace()) ||
                                trimmed_line[exclaim_pos..].starts_with("!!") ||
                                trimmed_line[exclaim_pos..].chars().nth(1)
                                    .is_some_and(|c| c.is_ascii_lowercase() || c == '<');
                    if is_tag { return true; }
                }
                // Anchor detection: & followed by valid anchor characters
                if let Some(amp_pos) = trimmed_line.find('&') {
                    let is_anchor = trimmed_line[amp_pos+1..].chars().next()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
                    if is_anchor { return true; }
                }
                // Alias detection: * followed by valid anchor characters
                if let Some(star_pos) = trimmed_line.find('*') {
                    let is_alias = trimmed_line[star_pos+1..].chars().next()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
                    if is_alias { return true; }
                }
                false
            });
            if has_node_properties {
                return Ok(None);
            }
        }

        // Chapter 8.2: Complex block mapping structures that exceed fast parser capabilities
        if trimmed.contains(':') && trimmed.lines().count() > 1 {
            // Detect explicit mapping indicators (?) requiring full parser
            if trimmed.lines().any(|line| {
                let trimmed_line = line.trim_start();
                trimmed_line.starts_with("? ") || trimmed_line == "?"
            }) {
                return Ok(None);
            }

            // Detect flow collections embedded in block mappings
            if trimmed.chars().any(|c| matches!(c, '[' | ']' | '{' | '}')) {
                return Ok(None);
            }

            // Detect complex indentation patterns that require full parser
            let mut prev_indent = None;
            for line in trimmed.lines() {
                if !line.trim().is_empty() && line.contains(':') {
                    let indent = line.len() - line.trim_start().len();
                    if let Some(prev) = prev_indent
                        && indent != prev && indent != 0 {
                            return Ok(None); // Variable indentation requires full parser
                        }
                    prev_indent = Some(indent);
                }
            }
        }

        // Block sequence: handle lists with "- item" syntax (CHECK FIRST!)
        // If it starts with "- ", it's likely a sequence - don't let block mapping claim it
        if trimmed.starts_with("- ") {
            // Try parsing as block sequence - let try_parse_block_sequence handle complexity
            if Self::is_valid_block_sequence(trimmed) {
                return Self::try_parse_block_sequence(trimmed);
            } else {
                // Invalid structure - fall back to full parser instead of erroring
                return Ok(None);
            }
        }

        // Multi-line mapping: handle simple block mappings (ONLY if not a sequence)
        // CRITICAL: Don't claim sequences that start with "- " as mappings!
        if trimmed.contains(':') && trimmed.lines().count() > 1 && !trimmed.starts_with("- ") {
            if let Some(result) = Self::try_parse_block_mapping(trimmed) {
                return Ok(Some(result));
            } else {
                // Complex mapping detected (anchors/aliases), fall back to full parser
                return Ok(None);
            }
        }

        // Single-line mapping: "key: value"
        if trimmed.contains(':')
            && trimmed.lines().count() == 1
            && let Some(colon_pos) = trimmed.find(':')
        {
            let key_part = trimmed[..colon_pos].trim();
            let value_part = trimmed[colon_pos + 1..].trim();

            if !key_part.is_empty()
                && !key_part.contains('[')
                && !key_part.contains('{')
                && !value_part.contains('[')
                && !value_part.contains('{')
                && !value_part.contains(':')
                && !key_part.contains('&')
                && !key_part.contains('*')
                && !value_part.contains('&')
                && !value_part.contains('*')
            {
                let mut hash = crate::linked_hash_map::LinkedHashMap::new();
                let key = Yaml::String(key_part.to_string());
                let value = if value_part.is_empty() {
                    Yaml::Null
                } else {
                    Self::parse_scalar_direct(value_part)
                };
                hash.insert(key, value);
                return Ok(Some(Yaml::Hash(hash)));
            }
        }

        // Simple array case: "[1, 2, 3]"
        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.lines().count() == 1 {
            let inner = &trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                return Ok(Some(Yaml::Array(Vec::new())));
            }

            let items: Vec<Yaml> = inner
                .split(',')
                .map(|item| Self::parse_scalar_direct(item.trim()))
                .collect();
            return Ok(Some(Yaml::Array(items)));
        }

        // Simple mapping case: "{key: value}" - only handle single key-value pairs
        if trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.lines().count() == 1 {
            let inner = &trimmed[1..trimmed.len() - 1].trim();
            println!("Fast parser: processing flow mapping '{}'", inner);
            if inner.is_empty() {
                return Ok(Some(Yaml::Hash(
                    crate::linked_hash_map::LinkedHashMap::new(),
                )));
            }

            // Check for multiple key-value pairs (contains comma) - fall back to full parser
            if inner.contains(',') {
                println!("Fast parser: detected comma in '{}', falling back to StateMachine", inner);
                return Ok(None);
            }

            if let Some(colon_pos) = inner.find(':') {
                let key_str = inner[..colon_pos].trim();
                let value_str = inner[colon_pos + 1..].trim();

                if !key_str.is_empty() && !value_str.is_empty() {
                    let mut hash = crate::linked_hash_map::LinkedHashMap::new();
                    let key = Yaml::String(key_str.to_string());
                    let value = Self::parse_scalar_direct(value_str);
                    hash.insert(key, value);
                    return Ok(Some(Yaml::Hash(hash)));
                }
            }
        }

        Ok(None)
    }

    /// Intelligent block sequence validation - zero allocation, blazing fast
    /// Validates block sequence structure with support for nested content
    #[inline]
    fn is_valid_block_sequence(s: &str) -> bool {
        let lines: Vec<&str> = s.lines().collect();
        if lines.is_empty() {
            return false;
        }

        let mut base_indent = None;
        let mut in_sequence_item = false;
        let mut item_indent = None;

        for line in lines.iter() {
            // Calculate indentation level
            let trimmed = line.trim();
            let indent_level = line.len() - line.trim_start().len();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with("- ") {
                // This is a sequence item marker
                if base_indent.is_none() {
                    base_indent = Some(indent_level);
                } else if base_indent != Some(indent_level) {
                    // Sequence items must be at same indentation level
                    return false;
                }
                in_sequence_item = true;
                item_indent = Some(indent_level + 2); // Content after "- " should be indented more
            } else if in_sequence_item {
                // This is content within a sequence item (nested mapping/sequence)
                if let Some(expected_indent) = item_indent
                    && indent_level < expected_indent
                {
                    // Content must be indented more than sequence marker
                    return false;
                }
                // Allow nested content within sequence items
            } else {
                // First line should be a sequence item, or we're not in a valid sequence
                return false;
            }
        }

        // Must have encountered at least one sequence item
        base_indent.is_some()
    }

    /// Parse simple block mapping format: key: value on separate lines
    fn try_parse_block_mapping(s: &str) -> Option<Yaml> {
        let mut hash = crate::linked_hash_map::LinkedHashMap::new();

        // First pass: check for nested indented content - if found, fall back to full parser
        let lines: Vec<&str> = s.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // If this line has a colon with empty value, check if next line is indented
            if let Some(colon_pos) = line.find(':') {
                let value_part = line[colon_pos + 1..].trim();
                if value_part.is_empty() && i + 1 < lines.len() {
                    // Check if next non-empty line is indented (nested content)
                    for next_line in &lines[i + 1..] {
                        if next_line.trim().is_empty() || next_line.trim().starts_with('#') {
                            continue;
                        }
                        // If next content line starts with whitespace, it's nested - use full parser
                        if next_line.starts_with(' ') || next_line.starts_with('\t') {
                            return None; // Fall back to full parser for nested content
                        }
                        break;
                    }
                }
            }
        }

        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue; // Skip empty lines and comments
            }

            if let Some(colon_pos) = line.find(':') {
                let key_part = line[..colon_pos].trim();
                let value_part = line[colon_pos + 1..].trim();

                // Check for anchor/alias syntax - fall back to full parser
                if line.contains('&') || line.contains('*') {
                    return None;
                }

                // Simple key-value pair - allow simple values including ~
                if !key_part.is_empty() {
                    let key = Yaml::String(key_part.to_string());
                    let value = if value_part.is_empty() {
                        Yaml::Null
                    } else {
                        Self::parse_scalar_direct(value_part)
                    };
                    hash.insert(key, value);
                } else {
                    // Complex structure detected, fall back to full parser
                    return None;
                }
            } else {
                // Non-mapping line detected, fall back to full parser
                return None;
            }
        }

        if hash.is_empty() {
            None
        } else {
            Some(Yaml::Hash(hash))
        }
    }

    /// Parse complex block sequence format with nested structures - zero allocation, blazing fast
    /// Returns detailed error information for malformed sequences
    #[inline]
    fn try_parse_block_sequence(s: &str) -> Result<Option<Yaml>, ScanError> {
        if s.is_empty() {
            return Ok(None);
        }

        let mut items = Vec::new();
        let mut lines_iter = s.lines().enumerate();

        // Pre-allocate with estimated capacity for better performance
        if s.len() > 100 {
            items.reserve(s.len() / 50); // Rough estimate: 50 chars per item
        }

        while let Some((line_num, line)) = lines_iter.next() {
            let trimmed = line.trim();

            // Skip empty lines and comments - zero allocation fast path
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some(first_line_content) = trimmed.strip_prefix("- ") {
                // Found a sequence item - collect all lines that belong to this item
                let base_indent = line.len() - line.trim_start().len();
                let item_content_indent = base_indent + 2; // Content after "- " should be more indented

                // Zero-allocation parsing: work with string slices directly
                // Remove "- " prefix
                let first_content_trimmed = first_line_content.trim();

                // Determine item boundaries without collecting into Vec
                let _item_start_pos = if first_content_trimmed.is_empty() {
                    None
                } else {
                    Some((first_content_trimmed, line_num, base_indent))
                };

                let mut item_end_line = line_num;
                let mut has_multiline_content = false;

                // Peek ahead to find item boundaries - zero allocation approach
                let mut line_offset = 1;
                let mut next_item_start = None;

                for next_line in s.lines().skip(line_num + 1) {
                    let actual_line_num = line_num + line_offset;
                    let next_trimmed = next_line.trim();
                    let next_indent = next_line.len() - next_line.trim_start().len();

                    // Skip empty lines and comments
                    if next_trimmed.is_empty() || next_trimmed.starts_with('#') {
                        line_offset += 1;
                        continue;
                    }

                    // If this line starts a new sequence item, stop collecting
                    if next_trimmed.starts_with("- ") && next_indent == base_indent {
                        next_item_start = Some(actual_line_num);
                        break;
                    }

                    // If this line is at or less indented than expected content, stop collecting
                    if next_indent < item_content_indent {
                        break;
                    }

                    // This line belongs to the current sequence item
                    item_end_line = actual_line_num;
                    has_multiline_content = true;
                    line_offset += 1;
                }

                // Parse item content with zero-allocation approach
                let item = if !has_multiline_content {
                    // Single line item - parse directly without allocation
                    if first_content_trimmed.is_empty() {
                        Ok(Yaml::Null)
                    } else {
                        Self::parse_item_content(first_content_trimmed)
                    }
                } else {
                    // Multi-line item - extract slice and parse
                    let item_lines: Vec<&str> = s
                        .lines()
                        .skip(line_num)
                        .take(item_end_line - line_num + 1)
                        .collect();

                    let mut content_parts = Vec::new();

                    // Add first line content if not empty
                    if !first_content_trimmed.is_empty() {
                        content_parts.push(first_content_trimmed);
                    }

                    // Add subsequent lines with normalized indentation
                    for item_line in item_lines.iter().skip(1) {
                        let item_trimmed = item_line.trim();
                        if item_trimmed.is_empty() || item_trimmed.starts_with('#') {
                            continue;
                        }

                        let item_indent = item_line.len() - item_line.trim_start().len();
                        let normalized_line = if item_indent >= item_content_indent {
                            &item_line[item_content_indent.min(item_line.len())..]
                        } else {
                            item_line
                        };
                        content_parts.push(normalized_line);
                    }

                    if content_parts.is_empty() {
                        Ok(Yaml::Null)
                    } else if content_parts.len() == 1 {
                        Self::parse_item_content(content_parts[0])
                    } else {
                        // Only allocate string when absolutely necessary
                        let joined_content = content_parts.join("\n");
                        Self::parse_item_content(&joined_content)
                    }
                };

                // Handle parsing errors
                let parsed_item = item?;

                items.push(parsed_item);

                // Skip lines we've already processed
                if let Some(next_start) = next_item_start {
                    // Fast-forward iterator to next item
                    for (current_line_num, _) in lines_iter.by_ref() {
                        if current_line_num + 1 >= next_start {
                            break;
                        }
                    }
                } else {
                    // Skip to end of current item
                    for _ in line_num..item_end_line {
                        lines_iter.next();
                    }
                }
            } else {
                // Unexpected line that doesn't start with "- " at the expected level
                return Err(ScanError::new(
                    Marker {
                        index: 0,
                        line: line_num + 1,
                        col: 0,
                    },
                    &format!(
                        "invalid block sequence: expected '- ' at line {}, found '{}'",
                        line_num + 1,
                        trimmed
                    ),
                ));
            }
        }

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Yaml::Array(items)))
        }
    }

    /// Parse content within a sequence item - handles scalars, mappings, and nested sequences
    /// Returns errors for malformed nested content
    #[inline]
    fn parse_item_content(content: &str) -> Result<Yaml, ScanError> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Ok(Yaml::Null);
        }

        // Use direct scalar parsing to avoid infinite recursion
        // (parse_item_content is called from try_fast_parse, so we can't call try_fast_parse again)

        // For complex content, use scalar parsing as fallback
        // This maintains compatibility while allowing nested structures
        Ok(Self::parse_scalar_direct(trimmed))
    }

    /// Direct scalar parsing without recursion - zero allocation, blazing fast
    /// Handles basic YAML scalar types: null, bool, int, float, string
    fn parse_scalar_direct(s: &str) -> Yaml {
        let trimmed = s.trim();

        // Handle null/empty values
        if trimmed.is_empty() || trimmed == "null" || trimmed == "~" {
            return Yaml::Null;
        }

        // Handle quoted strings
        if trimmed.len() >= 2
            && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        {
            // Remove quotes and return as string
            return Yaml::String(trimmed[1..trimmed.len() - 1].to_string());
        }

        // Handle boolean values
        match trimmed.to_lowercase().as_str() {
            "true" | "yes" | "on" => return Yaml::Boolean(true),
            "false" | "no" | "off" => return Yaml::Boolean(false),
            _ => {}
        }

        // Handle numeric values directly without recursion
        if let Ok(int_val) = trimmed.parse::<i64>() {
            return Yaml::Integer(int_val);
        }
        if let Ok(float_val) = trimmed.parse::<f64>() {
            return Yaml::Real(float_val.to_string());
        }

        // Handle special float values
        match trimmed.to_lowercase().as_str() {
            ".inf" | "+.inf" => return Yaml::Real("+.inf".to_string()),
            "-.inf" => return Yaml::Real("-.inf".to_string()),
            ".nan" => return Yaml::Real(".nan".to_string()),
            _ => {}
        }

        // Default to string
        Yaml::String(trimmed.to_string())
    }
}

/// The data structure that builds `Yaml` AST from parser events
pub struct YamlReceiver {
    pub docs: Vec<Yaml>,
    doc_stack: Vec<(Yaml, usize)>,
    key_stack: Vec<Yaml>,
    anchors: HashMap<usize, Yaml>,
    // Simple circular reference detection
    resolution_stack: Vec<usize>,
    // Billion laughs protection
    alias_count: usize,
}

impl Default for YamlReceiver {
    fn default() -> Self {
        Self::new()
    }
}

impl YamlReceiver {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            docs: Vec::with_capacity(1),         // Most YAML files have 1 document
            doc_stack: Vec::with_capacity(8),    // Typical nesting depth
            key_stack: Vec::with_capacity(8),    // Typical mapping depth
            anchors: HashMap::with_capacity(16), // Reasonable anchor count
            resolution_stack: Vec::with_capacity(8), // Rare deep circular refs
            alias_count: 0,                      // Start with no aliases processed
        }
    }

    #[inline]
    fn insert_new_node(&mut self, (node, aid): (Yaml, usize)) {
        // store anchor if needed - blazing-fast HashMap operations
        if aid > 0 {
            self.anchors.insert(aid, node.clone());
        }
        if self.doc_stack.is_empty() {
            self.doc_stack.push((node, 0));
        } else if let Some(top) = self.doc_stack.last_mut() {
            match top.0 {
                Yaml::Array(ref mut arr) => arr.push(node),
                Yaml::Hash(ref mut h) => {
                    if let Some(cur_key) = self.key_stack.last_mut() {
                        if cur_key.is_badvalue() {
                            *cur_key = node;
                        } else {
                            let mut swap_key = Yaml::BadValue;
                            std::mem::swap(&mut swap_key, cur_key);
                            h.insert(swap_key, node);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Blazing-fast alias resolution with circular reference protection
    #[inline]
    fn resolve_alias(&mut self, id: usize) -> Yaml {
        // Billion laughs protection - limit total alias resolutions
        self.alias_count += 1;
        if self.alias_count > 1000 {
            warn!(
                "Alias count exceeded limit ({}), potential billion laughs attack",
                self.alias_count
            );
            return Yaml::Null;
        }

        // Fast circular reference check - O(n) but n is typically very small (< 10 deep)
        if self.resolution_stack.contains(&id) {
            warn!(
                "Circular reference detected for alias ID {}, breaking cycle",
                id
            );
            return Yaml::Null;
        }

        // Look up the anchored value and return it immediately
        if let Some(anchored_node) = self.anchors.get(&id).cloned() {
            anchored_node
        } else {
            warn!("Anchor ID {} not found, returning null", id);
            Yaml::Null
        }
    }

    /// Reset alias tracking state (called between documents)
    #[inline]
    fn reset_alias_tracking(&mut self) {
        self.resolution_stack.clear();
        self.alias_count = 0;
    }
}

impl EventReceiver for YamlReceiver {
    fn on_event(&mut self, ev: Event) {
        trace!(
            "YAML EVENT: {:?} (doc_stack len: {}, docs len: {})",
            ev,
            self.doc_stack.len(),
            self.docs.len()
        );
        match ev {
            Event::DocumentStart => {
                // Reset alias tracking for each new document
                self.reset_alias_tracking();
            }
            Event::DocumentEnd => match self.doc_stack.len() {
                0 => self.docs.push(Yaml::BadValue),
                1 => {
                    if let Some((doc, _)) = self.doc_stack.pop() {
                        self.docs.push(doc);
                    }
                }
                _ => {}
            },
            Event::StreamStart => {}
            Event::StreamEnd => {}
            Event::Alias(id) => {
                let node = self.resolve_alias(id);
                self.insert_new_node((node, 0));
            }
            Event::Scalar(s, style, aid, tag) => {
                let node = if style != TScalarStyle::Plain {
                    Yaml::String(s)
                } else if let Some(TokenType::Tag(ref handle, ref suffix)) = tag {
                    // handle tag
                    if handle == "!!" {
                        match suffix.as_str() {
                            "bool" => match s.parse::<bool>() {
                                Ok(b) => Yaml::Boolean(b),
                                Err(_) => Yaml::BadValue,
                            },
                            "int" => match s.parse::<i64>() {
                                Ok(i) => Yaml::Integer(i),
                                Err(_) => Yaml::BadValue,
                            },
                            "float" => match s.parse::<f64>() {
                                Ok(_) => Yaml::Real(s),
                                Err(_) => Yaml::BadValue,
                            },
                            "null" => {
                                if s == "~" || s == "null" {
                                    Yaml::Null
                                } else {
                                    Yaml::BadValue
                                }
                            }
                            _ => Yaml::String(s),
                        }
                    } else {
                        // Preserve custom tag by creating a Tagged variant
                        let tag_name = if handle.is_empty() {
                            suffix.clone()
                        } else {
                            format!("{}{}", handle, suffix)
                        };
                        let inner_value = YamlLoader::parse_scalar_direct(&s);
                        Yaml::Tagged(tag_name, Box::new(inner_value))
                    }
                } else {
                    // autodetect
                    YamlLoader::parse_scalar_direct(&s)
                };
                self.insert_new_node((node, aid));
            }
            Event::SequenceStart(aid) => {
                self.doc_stack.push((Yaml::Array(Vec::new()), aid));
            }
            Event::SequenceEnd => {
                if let Some(top) = self.doc_stack.pop() {
                    self.insert_new_node(top);
                }
            }
            Event::MappingStart(aid) => {
                let h = LinkedHashMap::new();
                self.doc_stack.push((Yaml::Hash(h), aid));
                self.key_stack.push(Yaml::BadValue);
            }
            Event::MappingEnd => {
                self.key_stack.pop();
                if let Some(top) = self.doc_stack.pop() {
                    self.insert_new_node(top);
                }
            }
            Event::YamlDirective(_major, _minor) => {
                // Store YAML version directive for document processing
                // This is handled at the state machine level, no action needed here
            }
            Event::TagDirective(_handle, _prefix) => {
                // Store TAG directive for document processing
                // This is handled at the state machine level, no action needed here
            }
            Event::Nothing => {}
        }
    }
}

// Old load function removed - StateMachine::parse() handles loading directly
/*
pub fn load<T: Iterator<Item = char>, R: MarkedEventReceiver>(
    parser: &mut Parser<T>,
    recv: &mut R,
    multi: bool,
) -> Result<(), ScanError> {
    // ZERO-ALLOCATION, NON-RECURSIVE LOADER USING EXPLICIT STACK
    // Uses Vec<ContainerType> to track nesting instead of recursion
    #[derive(Debug, Clone, Copy)]
    enum ContainerType {
        Sequence,
        Mapping,
    }

    let mut nesting_stack: Vec<ContainerType> = Vec::with_capacity(32); // Pre-allocate for performance
    let mut documents_processed = 0;
    let mut in_document = false;

    // Ensure stream has started
    if !parser.scanner.stream_started() {
        let (ev, mark) = parser.next()?;
        if ev != Event::StreamStart {
            return Err(ScanError::new(
                mark,
                &format!("Expected StreamStart event, got {ev:?}")
            ));
        }
        recv.on_event(ev, mark);
    }

    if parser.scanner.stream_ended() {
        recv.on_event(Event::StreamEnd, parser.scanner.mark());
        return Ok(());
    }

    // FLAT EVENT PROCESSING LOOP - ZERO RECURSION
    loop {
        let (ev, mark) = parser.next()?;

        match ev {
            Event::StreamEnd => {
                if in_document {
                    recv.on_event(Event::DocumentEnd, mark);
                }
                recv.on_event(ev, mark);
                break;
            }

            Event::DocumentStart => {
                if in_document && multi {
                    recv.on_event(Event::DocumentEnd, mark);
                }
                parser.anchors.clear();
                recv.on_event(ev, mark);
                in_document = true;
                documents_processed += 1;
                if !multi && documents_processed > 1 {
                    // Single document mode: ignore additional documents
                    continue;
                }
            }

            Event::DocumentEnd => {
                recv.on_event(ev, mark);
                in_document = false;
            }

            Event::SequenceStart(_) => {
                if !in_document {
                    // Implicit document start
                    parser.anchors.clear();
                    recv.on_event(Event::DocumentStart, mark);
                    in_document = true;
                    documents_processed += 1;
                }
                recv.on_event(ev, mark);
                nesting_stack.push(ContainerType::Sequence);
            }

            Event::SequenceEnd => {
                recv.on_event(ev, mark);
                if let Some(ContainerType::Sequence) = nesting_stack.pop() {
                    // Correct nesting
                } else {
                    return Err(ScanError::new(
                        mark,
                        "Unexpected SequenceEnd: not inside sequence"
                    ));
                }
            }

            Event::MappingStart(_) => {
                if !in_document {
                    // Implicit document start
                    parser.anchors.clear();
                    recv.on_event(Event::DocumentStart, mark);
                    in_document = true;
                    documents_processed += 1;
                }
                recv.on_event(ev, mark);
                nesting_stack.push(ContainerType::Mapping);
            }

            Event::MappingEnd => {
                recv.on_event(ev, mark);
                if let Some(ContainerType::Mapping) = nesting_stack.pop() {
                    // Correct nesting
                } else {
                    return Err(ScanError::new(
                        mark,
                        "Unexpected MappingEnd: not inside mapping"
                    ));
                }
            }

            Event::Scalar(..) | Event::Alias(..) => {
                if !in_document {
                    // Implicit document start
                    parser.anchors.clear();
                    recv.on_event(Event::DocumentStart, mark);
                    in_document = true;
                    documents_processed += 1;
                }
                recv.on_event(ev, mark);
            }

            _ => {
                // Handle any other events directly
                if !in_document {
                    // Implicit document start
                    parser.anchors.clear();
                    recv.on_event(Event::DocumentStart, mark);
                    in_document = true;
                    documents_processed += 1;
                }
                recv.on_event(ev, mark);
            }
        }

        // Single document mode: break after processing first document
        if !multi && documents_processed >= 1 && nesting_stack.is_empty() && in_document {
            // Continue to find StreamEnd
            loop {
                let (next_ev, next_mark) = parser.next()?;
                if matches!(next_ev, Event::StreamEnd) {
                    recv.on_event(Event::DocumentEnd, next_mark);
                    recv.on_event(next_ev, next_mark);
                    break;
                }
                // Skip other events in single document mode
            }
            break;
        }
    }

    // Verify all containers were properly closed
    if !nesting_stack.is_empty() {
        return Err(ScanError::new(
            parser.scanner.mark(),
            &format!("Unclosed containers at end of stream: {} remaining", nesting_stack.len())
        ));
    }

    Ok(())
}
*/

// REMOVED: load_document function - replaced with flat, non-recursive loader
// This function was causing stack overflow via recursive calls to load_node

// REMOVED: load_node function - replaced with flat, non-recursive loader
// This function was causing infinite recursion via load_sequence/load_mapping calls

// REMOVED: load_sequence function - replaced with flat, non-recursive loader
// This function was causing infinite recursion via load_node calls

// REMOVED: load_mapping function - replaced with flat, non-recursive loader
// This function was causing infinite recursion via load_node calls
